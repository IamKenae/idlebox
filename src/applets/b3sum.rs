use std::fs::File;
use std::io::{self, BufReader, Read};
#[cfg(unix)]
use std::os::unix::fs::FileExt;
#[cfg(unix)]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(unix)]
use std::sync::Arc;
#[cfg(unix)]
use std::thread;

use crate::applets::hash_common::{hex_encode, run_hash_applet, HashImpl};
use crate::core::Applet;

pub struct B3sumApplet;

impl Applet for B3sumApplet {
    fn name(&self) -> &'static str {
        "b3sum"
    }

    fn description(&self) -> &'static str {
        "Compute and check BLAKE3 message digest (parallelized)"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        run_hash_applet::<Blake3Hasher>("b3sum", args)
    }
}

const IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

const MSG_PERMUTATION: [usize; 16] = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];

const CHUNK_START: u32 = 1 << 0;
const CHUNK_END: u32 = 1 << 1;
const PARENT: u32 = 1 << 2;
const ROOT: u32 = 1 << 3;

fn quarter_round(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, m0: u32, m1: u32) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m0);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(12);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m1);
    v[d] = (v[d] ^ v[a]).rotate_right(8);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(7);
}

fn compress(
    cv: &[u32; 8],
    block: &[u8; 64],
    counter: u64,
    block_len: u32,
    flags: u32,
) -> [u32; 16] {
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u32::from_le_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
    }

    let mut v = [0u32; 16];
    v[0..8].copy_from_slice(cv);
    v[8..12].copy_from_slice(&IV[0..4]);
    v[12] = counter as u32;
    v[13] = (counter >> 32) as u32;
    v[14] = block_len;
    v[15] = flags;

    let mut next_m = [0u32; 16];
    for _ in 0..7 {
        quarter_round(&mut v, 0, 4, 8, 12, m[0], m[1]);
        quarter_round(&mut v, 1, 5, 9, 13, m[2], m[3]);
        quarter_round(&mut v, 2, 6, 10, 14, m[4], m[5]);
        quarter_round(&mut v, 3, 7, 11, 15, m[6], m[7]);

        quarter_round(&mut v, 0, 5, 10, 15, m[8], m[9]);
        quarter_round(&mut v, 1, 6, 11, 12, m[10], m[11]);
        quarter_round(&mut v, 2, 7, 8, 13, m[12], m[13]);
        quarter_round(&mut v, 3, 4, 9, 14, m[14], m[15]);

        for i in 0..16 {
            next_m[i] = m[MSG_PERMUTATION[i]];
        }
        m = next_m;
    }
    v
}

struct ChunkState {
    cv: [u32; 8],
    chunk_counter: u64,
    block: [u8; 64],
    block_len: u8,
    blocks_compressed: u8,
    flags: u32,
}

impl ChunkState {
    fn new(key: [u32; 8], chunk_counter: u64, flags: u32) -> Self {
        Self {
            cv: key,
            chunk_counter,
            block: [0; 64],
            block_len: 0,
            blocks_compressed: 0,
            flags,
        }
    }

    fn update(&mut self, mut input: &[u8]) {
        while !input.is_empty() {
            if self.block_len as usize == 64 {
                let mut block_flags = self.flags;
                if self.blocks_compressed == 0 {
                    block_flags |= CHUNK_START;
                }
                let v = compress(&self.cv, &self.block, self.chunk_counter, 64, block_flags);
                for i in 0..8 {
                    self.cv[i] = v[i] ^ v[i + 8];
                }
                self.blocks_compressed += 1;
                self.block = [0; 64];
                self.block_len = 0;
            }

            let take = (64 - self.block_len as usize).min(input.len());
            self.block[self.block_len as usize..self.block_len as usize + take]
                .copy_from_slice(&input[..take]);
            self.block_len += take as u8;
            input = &input[take..];
        }
    }

    fn output(&self) -> Output {
        let mut block_flags = self.flags | CHUNK_END;
        if self.blocks_compressed == 0 {
            block_flags |= CHUNK_START;
        }
        Output {
            input_cv: self.cv,
            counter: self.chunk_counter,
            block: self.block,
            block_len: self.block_len as u32,
            flags: block_flags,
        }
    }
}

struct Output {
    input_cv: [u32; 8],
    counter: u64,
    block: [u8; 64],
    block_len: u32,
    flags: u32,
}

impl Output {
    fn chaining_value(&self) -> [u32; 8] {
        let v = compress(
            &self.input_cv,
            &self.block,
            self.counter,
            self.block_len,
            self.flags,
        );
        let mut cv = [0u32; 8];
        for i in 0..8 {
            cv[i] = v[i] ^ v[i + 8];
        }
        cv
    }

    fn root_bytes(&self) -> [u8; 32] {
        let v = compress(
            &self.input_cv,
            &self.block,
            self.counter,
            self.block_len,
            self.flags | ROOT,
        );
        let mut out = [0u8; 32];
        for i in 0..8 {
            out[i * 4..i * 4 + 4].copy_from_slice(&(v[i] ^ v[i + 8]).to_le_bytes());
        }
        out
    }
}

fn parent_output(left: &[u32; 8], right: &[u32; 8], key: [u32; 8], flags: u32) -> Output {
    let mut block = [0u8; 64];
    for i in 0..8 {
        block[i * 4..i * 4 + 4].copy_from_slice(&left[i].to_le_bytes());
        block[(i + 8) * 4..(i + 8) * 4 + 4].copy_from_slice(&right[i].to_le_bytes());
    }
    Output {
        input_cv: key,
        counter: 0,
        block,
        block_len: 64,
        flags: flags | PARENT,
    }
}

fn parent_cv(left: &[u32; 8], right: &[u32; 8], key: [u32; 8], flags: u32) -> [u32; 8] {
    parent_output(left, right, key, flags).chaining_value()
}

struct Blake3Hasher {
    chunk_state: ChunkState,
    cv_stack: Vec<([u32; 8], u8)>,
}

impl HashImpl for Blake3Hasher {
    fn new() -> Self {
        Self {
            chunk_state: ChunkState::new(IV, 0, 0),
            cv_stack: Vec::new(),
        }
    }

    fn update(&mut self, mut input: &[u8]) {
        while !input.is_empty() {
            let chunk_bytes_processed = self.chunk_state.blocks_compressed as usize * 64
                + self.chunk_state.block_len as usize;
            if chunk_bytes_processed == BYTES_PER_CHUNK {
                let cv = self.chunk_state.output().chaining_value();
                self.push_cv_at_height(cv, 0);
                self.chunk_state = ChunkState::new(
                    IV,
                    self.chunk_state.chunk_counter + 1,
                    self.chunk_state.flags,
                );
            }

            let chunk_bytes_processed = self.chunk_state.blocks_compressed as usize * 64
                + self.chunk_state.block_len as usize;
            let take = (BYTES_PER_CHUNK - chunk_bytes_processed).min(input.len());
            self.chunk_state.update(&input[..take]);
            input = &input[take..];
        }
    }

    fn finalize(self) -> String {
        let mut output = self.chunk_state.output();
        for (cv, _) in self.cv_stack.into_iter().rev() {
            let right_cv = output.chaining_value();
            output = parent_output(&cv, &right_cv, IV, 0);
        }

        let root_bytes = output.root_bytes();
        hex_encode(&root_bytes)
    }

    fn hash_file(file: &str) -> io::Result<String> {
        if file == "-" {
            let mut hasher = Blake3Hasher::new();
            let mut reader = io::stdin();
            let mut buf = [0u8; 32 * 1024];
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
            }
            return Ok(hasher.finalize());
        }

        let f = File::open(file)?;
        let meta = f.metadata()?;
        #[allow(unused_variables)]
        let size = meta.len();

        #[cfg(unix)]
        if size > 1024 * 1024 {
            return hash_file_parallel(f, size);
        }

        let mut hasher = Blake3Hasher::new();
        let mut reader = BufReader::new(f);
        let mut buf = [0u8; 32 * 1024];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Ok(hasher.finalize())
    }
}

impl Blake3Hasher {
    fn push_cv_at_height(&mut self, mut cv: [u32; 8], mut height: u8) {
        while let Some(&(prev_cv, prev_height)) = self.cv_stack.last() {
            if prev_height == height {
                self.cv_stack.pop();
                cv = parent_cv(&prev_cv, &cv, IV, 0);
                height += 1;
            } else {
                break;
            }
        }
        self.cv_stack.push((cv, height));
    }
}

// Number of bytes per chunk
const BYTES_PER_CHUNK: usize = 1024;
#[cfg(unix)]
const CHUNKS_PER_1MB: usize = 1024 * 1024 / BYTES_PER_CHUNK;
// Max tree height for parallel processing
#[cfg(unix)]
const TREE_HEIGHT: u8 = 10;

// We use `read_exact_at` for lock-free parallel I/O. Since this is an extension
// trait in std::os::unix, this parallel path is only enabled on Unix systems.
// On Windows, it gracefully falls back to the standard sequential processing in hash_file.
#[cfg(unix)]
fn hash_file_parallel(f: File, size: u64) -> io::Result<String> {
    let chunk_size = 1024 * 1024;
    let num_full_blocks = (size / chunk_size) as usize;

    let current_block = Arc::new(AtomicUsize::new(0));
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(num_full_blocks);

    let mut handles = Vec::new();

    for _ in 0..num_threads {
        let current_block = current_block.clone();
        let f = f.try_clone()?;
        handles.push(thread::spawn(
            move || -> io::Result<Vec<(usize, [u32; 8])>> {
                let mut results = Vec::new();
                let mut buf = vec![0u8; chunk_size as usize];
                loop {
                    let idx = current_block.fetch_add(1, Ordering::Relaxed);
                    if idx >= num_full_blocks {
                        break;
                    }

                    f.read_exact_at(&mut buf, (idx as u64) * chunk_size)?;

                    let mut local_hasher = Blake3Hasher::new();
                    local_hasher.chunk_state.chunk_counter = (idx as u64) * (CHUNKS_PER_1MB as u64);
                    local_hasher.update(&buf);

                    let mut output = local_hasher.chunk_state.output();
                    for (cv, _) in local_hasher.cv_stack.drain(..).rev() {
                        let right_cv = output.chaining_value();
                        output = parent_output(&cv, &right_cv, IV, 0);
                    }
                    let cv = output.chaining_value();

                    results.push((idx, cv));
                }
                Ok(results)
            },
        ));
    }

    let mut all_cvs = vec![[0u32; 8]; num_full_blocks];
    for handle in handles {
        let results = handle
            .join()
            .map_err(|_| io::Error::other("thread panicked"))??;
        for (idx, cv) in results {
            all_cvs[idx] = cv;
        }
    }

    let mut main_hasher = Blake3Hasher::new();
    for cv in all_cvs {
        main_hasher.push_cv_at_height(cv, TREE_HEIGHT);
    }

    main_hasher.chunk_state.chunk_counter = (num_full_blocks as u64) * (CHUNKS_PER_1MB as u64);

    let rem = size % chunk_size;
    if rem > 0 {
        let mut buf = vec![0u8; rem as usize];
        f.read_exact_at(&mut buf, size - rem)?;
        main_hasher.update(&buf);
    }

    Ok(main_hasher.finalize())
}
