use crate::error::{AetherError, Result};
use std::collections::HashSet;
use std::net::TcpListener;
use std::sync::Mutex;

pub struct PortAllocator {
    inner: Mutex<PortAllocatorInner>,
}

struct PortAllocatorInner {
    reserved: HashSet<u16>,
}

impl PortAllocator {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PortAllocatorInner {
                reserved: HashSet::new(),
            }),
        }
    }

    pub fn allocate(&self, count: usize) -> Result<Vec<u16>> {
        let mut inner = self.inner.lock().unwrap();
        let mut allocated = Vec::new();

        for _ in 0..count {
            let port = Self::find_free_port()?;
            inner.reserved.insert(port);
            allocated.push(port);
        }

        Ok(allocated)
    }

    fn find_free_port() -> Result<u16> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| AetherError::PortAllocation(format!("Failed to bind: {}", e)))?;
        let port = listener.local_addr()?.port();
        Ok(port)
    }

    pub fn release(&self, ports: &[u16]) {
        let mut inner = self.inner.lock().unwrap();
        for port in ports {
            inner.reserved.remove(port);
        }
    }
}

impl Default for PortAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_single_port() {
        let allocator = PortAllocator::new();
        let ports = allocator.allocate(1).unwrap();
        assert_eq!(ports.len(), 1);
        assert!(ports[0] > 0);
    }

    #[test]
    fn test_allocate_multiple_ports() {
        let allocator = PortAllocator::new();
        let ports = allocator.allocate(5).unwrap();
        assert_eq!(ports.len(), 5);

        let unique: HashSet<u16> = ports.iter().copied().collect();
        assert_eq!(unique.len(), 5); // All unique
    }

    #[test]
    fn test_release_ports() {
        let allocator = PortAllocator::new();
        let ports = allocator.allocate(2).unwrap();
        allocator.release(&ports);

        // Verify internal state is updated
        let inner = allocator.inner.lock().unwrap();
        assert!(inner.reserved.is_empty());
    }
}
