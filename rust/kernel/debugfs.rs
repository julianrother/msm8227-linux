#![allow(missing_docs)]
// SPDX-License-Identifier: GPL-2.0

//! API to add files to debugfs.
//!
//! C header: [`include/linux/debugfs.h`](../../../../include/linux/debugfs.h)
//!
//! Reference: <https://www.kernel.org/doc/html/next/filesystems/debugfs.html>

use crate::error::{from_err_ptr, Result};
use crate::fs::file;
use crate::prelude::*;
use crate::str::CStr;
use crate::sync::Arc;
use crate::types::Mode;
use core::ffi::c_void;

pub type PinnedRegistration<T = ()> = Pin<KBox<Registration<T>>>;

/// A registration of a debugfs directory or file
pub struct Registration<T> {
    open_data: T,
    dentry: *mut bindings::dentry,
    _parent: Option<Arc<Registration<()>>>, // Store parent to prevent it from being dropped
}

// SAFETY: dentry is only being held by the struct and is not shared with anyone else, so if T is
// Send, it is safe to send this struct to another thread.
unsafe impl<T: Send> Send for Registration<T> {}

// SAFETY: dentry is never accessed except in Registration::drop. As long as T is Sync, then
// it is safe for Registration to be Sync
unsafe impl<T: Sync> Sync for Registration<T> {}

impl<T> Drop for Registration<T> {
    fn drop(&mut self) {
        // SAFETY: self.dentry is valid by the type invariant.
        unsafe {
            bindings::debugfs_remove(self.dentry);
        }
    }
}

impl Registration<()> {
    pub fn register_symlink(
        name: &'static CStr,
        parent: Option<Arc<Registration<()>>>,
        dest: &'static CStr,
    ) -> Result<Registration<()>> {
        let parent_dentry = parent.as_ref().map_or(core::ptr::null_mut(), |r| r.dentry);

        let dentry = from_err_ptr(unsafe {
            bindings::debugfs_create_symlink(name.as_char_ptr(), parent_dentry, dest.as_char_ptr())
        })?;

        Ok(Self {
            dentry,
            open_data: (),
            _parent: parent,
        })
    }

    pub fn register_dir(
        name: &'static CStr,
        parent: Option<Arc<Registration<()>>>,
    ) -> Result<Registration<()>> {
        let parent_dentry = parent.as_ref().map_or(core::ptr::null_mut(), |r| r.dentry);

        // SAFETY: name.as_char_ptr() cannot be null. The type invariant ensure that
        // self.dentry is always a valid pointer, so p will always be a NULL pointer or a valid
        // pointer.
        let dentry = from_err_ptr(unsafe {
            bindings::debugfs_create_dir(name.as_char_ptr(), parent_dentry)
        })?;

        Ok(Self {
            dentry,
            open_data: (),
            _parent: parent,
        })
    }
}

impl<T: Sync> Registration<T> {
    pub fn register_file<U>(
        name: &'static CStr,
        mode: Mode,
        open_data: T,
        parent: Option<Arc<Registration<()>>>,
    ) -> Result<PinnedRegistration<T>>
    where
        Self: file::OpenAdapter<T>,
        U: file::Operations<OpenData = T>,
    {
        let fops = unsafe { file::OperationsVtable::<Self, U>::build() };
        let parent_dentry = parent.as_ref().map_or(core::ptr::null_mut(), |r| r.dentry);

        let mut registration = Pin::from(KBox::new(
            Self {
                dentry: core::ptr::null_mut(),
                open_data,
                _parent: parent,
            },
            GFP_KERNEL,
        )?);
        // SAFETY: The function never moves `this` hence the call is safe.
        let this = unsafe { registration.as_mut().get_unchecked_mut() };
        this.dentry = from_err_ptr(unsafe {
            bindings::debugfs_create_file_unsafe(
                name.as_char_ptr(),
                mode.as_int(),
                parent_dentry,
                this as *mut _ as *mut c_void,
                fops,
            )
        })?;

        Ok(registration)
    }
}

impl<T: Sync> file::OpenAdapter<T> for Registration<T> {
    // WIP: Returns a valid pointer that lives longer than the call to the open function
    unsafe fn convert(inode: *mut bindings::inode, _file: *mut bindings::file) -> *const T {
        // SAFETY: debugfs_create_file is called with self as private data. The C debugfs API
        // stores it into the inode.i_private field.
        let this: &Self = unsafe { &*((*inode).i_private as *const Self) };
        &this.open_data
    }
}
