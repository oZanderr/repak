/// Called with the pak's mount point and the file's relative path to determine how many
/// leading bytes of the (possibly compressed) file data to encrypt. Return `total_len` to
/// encrypt the entire file (the default), or a smaller value for partial encryption.
pub type EncryptPrefixFn = fn(mount_point: &str, path: &str, total_len: usize) -> usize;

#[derive(Debug, Clone, Copy)]
pub struct PakProfile {
    pub encrypt_prefix: EncryptPrefixFn,
    pub reverse_word_order: bool,
    pub index_trailer: &'static [u8],
}

fn default_encrypt_prefix(_mount_point: &str, _path: &str, total_len: usize) -> usize {
    total_len
}

impl Default for PakProfile {
    fn default() -> Self {
        Self {
            encrypt_prefix: default_encrypt_prefix,
            reverse_word_order: false,
            index_trailer: &[],
        }
    }
}

pub fn normalize_joined_path(mount_point: &str, path: &str) -> String {
    let path = format!("{}/{}", mount_point, path);

    let mut last = false;
    path.chars()
        .filter(|&c| {
            let keep = c != '/' || !last;
            last = c == '/';
            keep
        })
        .collect::<String>()
}
