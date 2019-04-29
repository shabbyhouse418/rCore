//! File handle for process

use alloc::{string::String, sync::Arc};

use crate::rcore_fs::vfs::{FsError, INode, Metadata, Result, VirtualFS, INodeContainer};
use crate::lkm::cdev::FileOperations;
#[derive(Clone)]
pub struct FileHandle {
    pub inode_container: Arc<INodeContainer>,
    offset: u64,
    options: OpenOptions,
    overlay_file_operations: Option<Arc<FileOperations>>
}

#[derive(Debug, Clone)]
pub struct OpenOptions {
    pub read: bool,
    pub write: bool,
    /// Before each write, the file offset is positioned at the end of the file.
    pub append: bool,
}

#[derive(Debug)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}
macro_rules! overlay_op{
    ($sel:ident,$v:ident => $($x:expr),* ) => {
        {
            if let Some(overlay_file_operations)=$sel.overlay_file_operations.as_ref(){
                if let Some(funct)=overlay_file_operations.$v.as_ref(){
                    return funct($($x),*);
                }

            }
        }
    };
}
impl FileHandle {
    pub fn new(inode_container: Arc<INodeContainer>, options: OpenOptions) -> Self {
        FileHandle {
            inode_container,
            offset: 0,
            options,
            overlay_file_operations: None
        }
    }
    pub fn new_with_overlay_op(inode_container: Arc<INodeContainer>, options: OpenOptions, ops: Arc<FileOperations>)->Self{
        let mut handle=FileHandle::new(inode_container, options);
        handle.overlay_file_operations=Some(ops);
        (ops.open)(&mut handle);
        handle

    }
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        overlay_op!(self,read=>self, buf);
        let len = self.read_at(self.offset as usize, buf)?;
        self.offset += len as u64;
        Ok(len)
    }

    pub fn read_at(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        overlay_op!(self,read_at=>self, offset, buf);
        if !self.options.read {
            return Err(FsError::InvalidParam); // FIXME: => EBADF
        }
        let len = self.inode_container.inode.read_at(offset, buf)?;
        Ok(len)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        overlay_op!(self,write=>self, buf);
        let offset = match self.options.append {
            true => self.inode_container.inode.metadata()?.size as u64,
            false => self.offset,
        } as usize;
        let len = self.write_at(offset, buf)?;
        self.offset = (offset + len) as u64;
        Ok(len)
    }

    pub fn write_at(&mut self, offset: usize, buf: &[u8]) -> Result<usize> {
        overlay_op!(self,write_at=>self, offset, buf);
        if !self.options.write {
            return Err(FsError::InvalidParam); // FIXME: => EBADF
        }
        let len = self.inode_container.inode.write_at(offset, buf)?;
        Ok(len)
    }

    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        overlay_op!(self,seek=>self, pos);
        self.offset = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => (self.inode_container.inode.metadata()?.size as i64 + offset) as u64,
            SeekFrom::Current(offset) => (self.offset as i64 + offset) as u64,
        };
        Ok(self.offset)
    }

    pub fn set_len(&mut self, len: u64) -> Result<()> {
        overlay_op!(self,set_len=>self, len);
        if !self.options.write {
            return Err(FsError::InvalidParam); // FIXME: => EBADF
        }
        self.inode_container.inode.resize(len as usize)?;
        Ok(())
    }

    pub fn sync_all(&mut self) -> Result<()> {
        overlay_op!(self,sync_all=>self);self.inode_container.inode.sync_all()
    }

    pub fn sync_data(&mut self) -> Result<()> {
        overlay_op!(self,sync_data=>self);self.inode_container.inode.sync_data()
    }

    pub fn metadata(&self) -> Result<Metadata> {
        overlay_op!(self,metadata=>self);
        self.inode_container.inode.metadata()
    }

    // Putting lookup_follow here is a terrible idea.
    // We move it to PathConfig
    /*
    pub fn lookup_follow(&self, path: &str, max_follow: usize) -> Result<Arc<INode>> {
        self.inode.lookup_follow(path, max_follow)
    }
    */

    pub fn read_entry(&mut self) -> Result<String> {
        overlay_op!(self,read_entry=>self);
        if !self.options.read {
            return Err(FsError::InvalidParam); // FIXME: => EBADF
        }
        let name = self.inode_container.inode.get_entry(self.offset as usize)?;
        self.offset += 1;
        Ok(name)
    }
}
