use crate::applets::hash_common::{hex_encode, run_hash_applet, HashImpl};
use crate::core::Applet;

pub struct Md5sumApplet;

impl Applet for Md5sumApplet {
    fn name(&self) -> &'static str {
        "md5sum"
    }

    fn description(&self) -> &'static str {
        "Compute and check MD5 message digest"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        run_hash_applet::<Md5Hasher>("md5sum", args)
    }
}

const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

struct Md5Hasher {
    state: [u32; 4],
    buffer: [u8; 64],
    len: u64,
}

impl HashImpl for Md5Hasher {
    fn new() -> Self {
        Md5Hasher {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476],
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
        self.update(&bit_len.to_le_bytes());

        let mut out = [0u8; 16];
        for i in 0..4 {
            out[i * 4..i * 4 + 4].copy_from_slice(&self.state[i].to_le_bytes());
        }

        hex_encode(&out)
    }
}

impl Md5Hasher {
    fn process_block(&mut self, block: [u8; 64]) {
        let mut w = [0u32; 16];
        for i in 0..16 {
            w[i] = u32::from_le_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | (!b & d), i),
                16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                48..=63 => (c ^ (b | !d), (7 * i) % 16),
                _ => unreachable!(),
            };

            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                a.wrapping_add(f)
                    .wrapping_add(K[i])
                    .wrapping_add(w[g])
                    .rotate_left(S[i]),
            );
            a = temp;
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
    }
}
