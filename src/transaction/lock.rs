use crossbeam::channel::{bounded, Receiver, Sender};

use crate::thread::StoppableChannel;

#[derive(Debug)]
pub enum LockStatus {
  Released,
  Read(usize),
  Write,
}

#[derive(Debug)]
pub struct PageLocker {
  blocked: Vec<Sender<()>>,
  status: LockStatus,
}
impl Default for PageLocker {
  fn default() -> Self {
    Self::new()
  }
}
impl PageLocker {
  pub fn new() -> Self {
    Self {
      status: LockStatus::Released,
      blocked: Default::default(),
    }
  }

  pub fn fetch_read(
    &mut self,
    index: usize,
    releaser: StoppableChannel<usize>,
  ) -> Result<PageLock, Receiver<()>> {
    match self.status {
      LockStatus::Released => {
        self.status = LockStatus::Read(1);
        return Ok(PageLock::new(releaser, index));
      }
      LockStatus::Read(count) => {
        self.status = LockStatus::Read(count + 1);
        return Ok(PageLock::new(releaser, index));
      }
      LockStatus::Write => {
        let (tx, rx) = bounded(1);
        self.blocked.push(tx);
        return Err(rx);
      }
    }
  }

  pub fn fetch_write(
    &mut self,
    index: usize,
    releaser: StoppableChannel<usize>,
  ) -> Result<PageLock, Receiver<()>> {
    if let LockStatus::Released = self.status {
      self.status = LockStatus::Write;
      return Ok(PageLock::new(releaser, index));
    }

    let (tx, rx) = bounded(1);
    self.blocked.push(tx);
    return Err(rx);
  }

  pub fn release(&mut self) -> Option<impl Iterator<Item = Sender<()>> + '_> {
    if let LockStatus::Read(count) = self.status {
      if count != 1 {
        self.status = LockStatus::Read(count.checked_sub(1).unwrap_or(0));
        return None;
      }
    }

    self.status = LockStatus::Released;
    if self.blocked.len() == 0 {
      return None;
    }

    return Some(self.blocked.drain(..));
  }
}
pub struct PageLock {
  index: usize,
  releaser: StoppableChannel<usize>,
}
impl PageLock {
  fn new(releaser: StoppableChannel<usize>, index: usize) -> Self {
    Self { releaser, index }
  }
}
impl Drop for PageLock {
  fn drop(&mut self) {
    self.releaser.send(self.index);
  }
}
