//! Old comments.
pub const FOO: i32 = 5;

#[cfg(unix)]
pub const PLATFORM: u8 = 0;
#[cfg(windows)]
pub const PLATFORM: u16 = 0;

pub mod wumbo {
    pub const W: () = ();
}
