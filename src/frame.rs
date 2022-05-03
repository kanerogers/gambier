use ash::vk;

use crate::{sync_structures::SyncStructures, vulkan_context::create_command_buffer};

pub struct Frame {
    pub command_buffer: vk::CommandBuffer,
    pub sync_structures: SyncStructures,
}

impl Frame {
    pub unsafe fn new(device: &ash::Device, command_pool: vk::CommandPool) -> Self {
        let command_buffer = create_command_buffer(&device, command_pool);
        let sync_structures = SyncStructures::new(&device);
        Self {
            sync_structures,
            command_buffer,
        }
    }
}
