use ash::{vk, Device, Instance};

use crate::buffer::Buffer;

pub struct Image {
    pub buffer: Buffer<u8>,
}

impl Image {
    pub unsafe fn new(
        device: &Device,
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        descriptor_pool: vk::DescriptorPool,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Self {
        // TODO: I can't remember if images have buffers or not!
        let buffer = Buffer::new(
            device,
            instance,
            physical_device,
            descriptor_pool,
            descriptor_set_layout,
            &[],
            vk::BufferUsageFlags::STORAGE_BUFFER,
        );

        Self { buffer }
    }
}
