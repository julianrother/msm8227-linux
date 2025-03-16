// SPDX-License-Identifier: GPL-2.0

//! Rust debugfs device sample

#![allow(missing_docs)]

use kernel::{
    c_str, debugfs,
    fs::file::{self, File},
    io_buffer::IoBufferWriter,
    prelude::*,
    sync::Arc,
    types::Mode,
};

struct SampleFile;

#[vtable]
impl file::Operations for SampleFile {
    fn open(_data: &(), _file: &File) -> Result {
        Ok(())
    }

    fn read(
        _data: (),
        _file: &File,
        writer: &mut impl IoBufferWriter,
        offset: u64,
    ) -> Result<usize> {
        let data = b"Sample debugfs file implementing file::Operations\n";
        let offset = offset as usize;

        if offset > data.len() {
            return Ok(0);
        }

        let len = core::cmp::min(writer.len(), data.len() - offset);
        writer.write_slice(&data[offset..(offset + len)])?;
        Ok(len)
    }
}

struct RustDebugfs {
    _sample_file: debugfs::PinnedRegistration,
    _symlink: debugfs::Registration<()>,
}
impl kernel::Module for RustDebugfs {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let dir = Arc::new(
            debugfs::Registration::register_dir(c_str!("rust_samples"), None)?,
            GFP_KERNEL,
        )?;

        let sample_file = debugfs::Registration::register_file::<SampleFile>(
            c_str!("sample"),
            Mode::from_int(0444),
            (),
            Some(dir.clone()),
        )?;

        let symlink = debugfs::Registration::register_symlink(
            c_str!("sample_symlink"),
            Some(dir.clone()),
            c_str!("sample"),
        )?;

        Ok(Self {
            _sample_file: sample_file,
            _symlink: symlink,
        })
    }
}

module! {
    type: RustDebugfs,
    name: "rust_debugfs",
    author: "Fabien Parent <fabien.parent@linaro.org>",
    description: "Rust debugfs sample",
    license: "GPL",
}
