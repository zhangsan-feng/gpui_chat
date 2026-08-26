pub const fn rgb_to_u32(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

pub fn avatar_source(source: &str, cache_key: &str) -> String {
    if source.is_empty()
        || cache_key.is_empty()
        || (!source.starts_with("http://") && !source.starts_with("https://"))
    {
        return source.to_string();
    }

    let separator = if source.contains('?') { '&' } else { '?' };
    format!("{source}{separator}avatar_key={cache_key}")
}

pub mod component;
pub mod home;
pub mod login;
