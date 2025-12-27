use fuser::{Filesystem, Request};
use libc::ENOENT;
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};

pub struct McFUSE;

impl Filesystem for McFUSE {
    // Тут мы будем переопределять методы: lookup, getattr, read, readdir...
}