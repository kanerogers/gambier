use ash::vk;

use crate::{buffer::Buffer, image::Image, vulkan_context::VulkanContext};

pub struct Texture {}

impl Texture {
    pub unsafe fn new(vulkan_context: &VulkanContext, image: &gltf::image::Data) -> Self {
        let data = &image.pixels;
        let device = &vulkan_context.device;
        let instance = &vulkan_context.instance;
        let physical_device = vulkan_context.physical_device;
        let descriptor_pool = vulkan_context.descriptor_pool;
        let descriptor_set_layout = vulkan_context.descriptor_set_layout;
        let extent = vk::Extent3D {
            width: image.width,
            height: image.height,
            depth: 1,
        };

        if image.format == gltf::image::Format::R8G8B8 {}

        let scratch_buffer = Buffer::new(
            device,
            instance,
            physical_device,
            descriptor_pool,
            descriptor_set_layout,
            data,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );

        let image = Image::new(
            device,
            instance,
            physical_device,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            extent,
        );

        Self {}
    }
}
