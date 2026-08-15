use crate::applets::hash_common::{hex_encode, run_hash_applet, HashImpl};
use crate::core::Applet;

pub struct Sha1sumApplet;

impl Applet for Sha1sumApplet {
    fn name(&self) -> &'static str {
        "sha1sum"
    }

    fn description(&self) -> &'static str {
        "Compute and check SHA1 message digest"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        run_hash_applet::<Sha1Hasher>("sha1sum", args)
    }
}

struct Sha1Hasher {
    state: [u32; 5],
    buffer: [u8; 64],
    len: u64,
}

impl HashImpl for Sha1Hasher {
    fn new() -> Self {
        Sha1Hasher {
            state: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
            buffer: [0; 64],
            len: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        let buffer_len = (self.len % 64) as usize;
        self.len += data.len() as u64;

        if buffer_len > 0 {
            let space = 64 - buffer_len;
            if data.len() < space {
                self.buffer[buffer_len..buffer_len + data.len()].copy_from_slice(data);
                return;
            }
            self.buffer[buffer_len..].copy_from_slice(&data[..space]);
            self.process_block(self.buffer);
            data = &data[space..];
        }

        while data.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[..64]);
            self.process_block(block);
            data = &data[64..];
        }

        if !data.is_empty() {
            self.buffer[..data.len()].copy_from_slice(data);
        }
    }

    fn finalize(mut self) -> String {
        let bit_len = self.len.wrapping_mul(8);
        self.update(&[0x80]);
        let buffer_len = (self.len % 64) as usize;
        if buffer_len > 56 {
            self.update(&[0; 64][..64 - buffer_len]);
        }
        let padding = 56 - (self.len % 64) as usize;
        self.update(&[0; 64][..padding]);
        self.update(&bit_len.to_be_bytes());

        let mut out = [0u8; 20];
        for i in 0..5 {
            out[i * 4..i * 4 + 4].copy_from_slice(&self.state[i].to_be_bytes());
        }

        hex_encode(&out)
    }
}

impl Sha1Hasher {
    fn process_block(&mut self, block: [u8; 64]) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];

        #[allow(clippy::needless_range_loop)]
        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                60..=79 => (b ^ c ^ d, 0xCA62C1D6),
                _ => unreachable!(),
            };

            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
    }
}
