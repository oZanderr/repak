
type Result<T, E = Error> = std::result::Result<T, E>;

pub use oodle_lz::{CompressionLevel, Compressor};

mod oodle_lz {
    #[derive(Debug, Clone, Copy)]
    #[repr(i32)]
    pub enum Compressor {
        /// None = memcpy, pass through uncompressed bytes
        None = 3,

        /// Fast decompression and high compression ratios, amazing!
        Kraken = 8,
        /// Leviathan = Kraken's big brother with higher compression, slightly slower decompression.
        Leviathan = 13,
        /// Mermaid is between Kraken & Selkie - crazy fast, still decent compression.
        Mermaid = 9,
        /// Selkie is a super-fast relative of Mermaid.  For maximum decode speed.
        Selkie = 11,
        /// Hydra, the many-headed beast = Leviathan, Kraken, Mermaid, or Selkie (see $OodleLZ_About_Hydra)
        Hydra = 12,
    }

    #[derive(Debug, Clone, Copy)]
    #[repr(i32)]
    pub enum CompressionLevel {
        /// don't compress, just copy raw bytes
        None = 0,
        /// super fast mode, lower compression ratio
        SuperFast = 1,
        /// fastest LZ mode with still decent compression ratio
        VeryFast = 2,
        /// fast - good for daily use
        Fast = 3,
        /// standard medium speed LZ mode
        Normal = 4,

        /// optimal parse level 1 (faster optimal encoder)
        Optimal1 = 5,
        /// optimal parse level 2 (recommended baseline optimal encoder)
        Optimal2 = 6,
        /// optimal parse level 3 (slower optimal encoder)
        Optimal3 = 7,
        /// optimal parse level 4 (very slow optimal encoder)
        Optimal4 = 8,
        /// optimal parse level 5 (don't care about encode speed, maximum compression)
        Optimal5 = 9,

        /// faster than SuperFast, less compression
        HyperFast1 = -1,
        /// faster than HyperFast1, less compression
        HyperFast2 = -2,
        /// faster than HyperFast2, less compression
        HyperFast3 = -3,
        /// fastest, less compression
        HyperFast4 = -4,
    }

    #[allow(non_snake_case)]
    pub type Compress = unsafe extern "system" fn(
        compressor: Compressor,
        rawBuf: *const u8,
        rawLen: usize,
        compBuf: *mut u8,
        level: CompressionLevel,
        pOptions: *const (),
        dictionaryBase: *const (),
        lrm: *const (),
        scratchMem: *mut u8,
        scratchSize: usize,
    ) -> isize;

    #[allow(non_snake_case)]
    pub type Decompress = unsafe extern "system" fn(
        compBuf: *const u8,
        compBufSize: usize,
        rawBuf: *mut u8,
        rawLen: usize,
        fuzzSafe: u32,
        checkCRC: u32,
        verbosity: u32,
        decBufBase: u64,
        decBufSize: usize,
        fpCallback: u64,
        callbackUserData: u64,
        decoderMemory: *mut u8,
        decoderMemorySize: usize,
        threadPhase: u32,
    ) -> isize;

    #[allow(non_snake_case)]
    pub type GetCompressedBufferSizeNeeded =
        unsafe extern "system" fn(compressor: Compressor, rawSize: usize) -> usize;

    pub type SetPrintf = unsafe extern "system" fn(printf: *const ());
}

struct OodlePlatform {
    name: &'static str,
    hash: &'static str,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
static OODLE_PLATFORM: OodlePlatform = OodlePlatform {
    name: "liboo2corelinux64.so.9",
    hash: "ed7e98f70be1254a80644efd3ae442ff61f854a2fe9debb0b978b95289884e9c",
};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
static OODLE_PLATFORM: OodlePlatform = OodlePlatform {
    name: "liboo2corelinuxarm64.so.9",
    hash: "161a8ecca8cc2d4ea6469779c2cc529ed5bb2765d99466273c29fdbef4657374",
};

#[cfg(all(target_os = "linux", target_arch = "arm"))]
static OODLE_PLATFORM: OodlePlatform = OodlePlatform {
    name: "liboo2corelinuxarm32.so.9",
    hash: "83cda016c033844fe650e49fac4cc19ff0a0fb4a3c9a7576a320ea39a9e4626b",
};

#[cfg(target_os = "macos")]
static OODLE_PLATFORM: OodlePlatform = OodlePlatform {
    name: "liboo2coremac64.2.9.10.dylib",
    hash: "b09af35f6b84a61e2b6488495c7927e1cef789b969128fa1c845e51a475ec501",
};

#[cfg(windows)]
static OODLE_PLATFORM: OodlePlatform = OodlePlatform {
    name: "oo2core_9_win64.dll",
    hash: "6f5d41a7892ea6b2db420f2458dad2f84a63901c9a93ce9497337b16c195f457",
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Oodle lib hash mismatch expected: {expected} got {found}")]
    HashMismatch { expected: String, found: String },
    #[error("Oodle compression failed")]
    CompressionFailed,
    #[error("Oodle decompression failed")]
    DecompressionFailed,
    #[error("IO error {0:?}")]
    Io(#[from] std::io::Error),
    #[error("Oodle library not found; place {name} beside the executable or set OODLE_LIB_PATH")]
    MissingLocalLibrary { name: &'static str },
    #[error("Oodle initialization failed previously: {cause}")]
    InitializationFailed { cause: String },
    #[error("Oodle libloading error {0:?}")]
    LibLoading(#[from] libloading::Error),
}

fn check_hash(buffer: &[u8]) -> Result<()> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(buffer);
    let hash = hex::encode(hasher.finalize());
    if hash != OODLE_PLATFORM.hash {
        return Err(Error::HashMismatch {
            expected: OODLE_PLATFORM.hash.into(),
            found: hash,
        });
    }

    Ok(())
}

fn resolve_oodle_path() -> Result<std::path::PathBuf> {
    let env_path = std::env::var("OODLE_LIB_PATH").ok().map(|p| {
        let p = std::path::PathBuf::from(p);
        if p.is_dir() { p.join(OODLE_PLATFORM.name) } else { p }
    });

    let path = env_path.into_iter()
        .chain(std::env::current_exe().ok().map(|e| e.with_file_name(OODLE_PLATFORM.name)))
        .chain(std::env::current_dir().ok().map(|d| d.join(OODLE_PLATFORM.name)))
        .find(|p| p.is_file())
        .ok_or(Error::MissingLocalLibrary { name: OODLE_PLATFORM.name })?;

    check_hash(&std::fs::read(&path)?)?;
    Ok(path)
}

pub struct Oodle {
    _library: libloading::Library,
    compress: oodle_lz::Compress,
    decompress: oodle_lz::Decompress,
    get_compressed_buffer_size_needed: oodle_lz::GetCompressedBufferSizeNeeded,
}
impl Oodle {
    fn new(lib: libloading::Library) -> Result<Self> {
        let res = unsafe {
            // Silence Oodle's default printf output; loaded and called once, no need to retain.
            let set_printf: oodle_lz::SetPrintf = *lib.get(b"OodleCore_Plugins_SetPrintf")?;
            set_printf(std::ptr::null());

            Oodle {
                compress: *lib.get(b"OodleLZ_Compress")?,
                decompress: *lib.get(b"OodleLZ_Decompress")?,
                get_compressed_buffer_size_needed: *lib
                    .get(b"OodleLZ_GetCompressedBufferSizeNeeded")?,
                _library: lib,
            }
        };
        Ok(res)
    }
    pub fn compress(
        &self,
        input: &[u8],
        compressor: Compressor,
        compression_level: CompressionLevel,
    ) -> Result<Vec<u8>> {
        let buffer_size = self.get_compressed_buffer_size_needed(compressor, input.len());
        let mut buffer = vec![0; buffer_size];

        let len = unsafe {
            (self.compress)(
                compressor,
                input.as_ptr(),
                input.len(),
                buffer.as_mut_ptr(),
                compression_level,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                0,
            )
        };

        if len == -1 {
            return Err(Error::CompressionFailed);
        }
        buffer.truncate(len as usize);

        Ok(buffer)
    }
    pub fn decompress(&self, input: &[u8], output: &mut [u8]) -> Result<usize> {
        let len = unsafe {
            (self.decompress)(
                input.as_ptr(),
                input.len(),
                output.as_mut_ptr(),
                output.len(),
                1,             // fuzzSafe
                1,             // checkCRC
                0,             // verbosity
                0,             // decBufBase
                0,             // decBufSize
                0,             // fpCallback
                0,             // callbackUserData
                std::ptr::null_mut(), // decoderMemory (let Oodle allocate)
                0,             // decoderMemorySize
                3,             // threadPhase (OodleLZ_Decode_ThreadPhaseAll)
            )
        };
        if len < 0 {
            return Err(Error::DecompressionFailed);
        }
        Ok(len as usize)
    }
    fn get_compressed_buffer_size_needed(
        &self,
        compressor: oodle_lz::Compressor,
        raw_buffer: usize,
    ) -> usize {
        unsafe { (self.get_compressed_buffer_size_needed)(compressor, raw_buffer) }
    }
}

fn load_oodle() -> Result<Oodle> {
    let path = resolve_oodle_path()?;
    let library = unsafe { libloading::Library::new(path)? };
    Oodle::new(library)
}

// LazyLock initialises on first access. The inner Result uses String for the error because
// Error is not Clone, and LazyLock needs to hand out &T from a shared static.
static OODLE: std::sync::LazyLock<std::result::Result<Oodle, String>> =
    std::sync::LazyLock::new(|| load_oodle().map_err(|e| e.to_string()));

pub fn oodle() -> Result<&'static Oodle> {
    OODLE.as_ref().map_err(|e| Error::InitializationFailed { cause: e.clone() })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_oodle() {
        let oodle = oodle().unwrap();

        let data = b"In tools and when compressing large inputs in one call, consider using
        $OodleXLZ_Compress_AsyncAndWait (in the Oodle2 Ext lib) instead to get parallelism. Alternatively,
        chop the data into small fixed size chunks (we recommend at least 256KiB, i.e. 262144 bytes) and
        call compress on each of them, which decreases compression ratio but makes for trivial parallel
        compression and decompression.";

        let buffer = oodle
            .compress(data, Compressor::Kraken, CompressionLevel::Optimal5)
            .unwrap();

        dbg!((data.len(), buffer.len()));

        let mut uncomp = vec![0; data.len()];
        oodle.decompress(&buffer, &mut uncomp).unwrap();

        assert_eq!(data[..], uncomp[..]);
    }
}
