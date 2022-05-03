use std::io::Read;

use ash::vk;
use image::{DynamicImage, ImageBuffer};

use crate::{buffer::Buffer, image::Image, vulkan_context::VulkanContext};

pub struct Texture {}

impl Texture {
    pub unsafe fn new(vulkan_context: &VulkanContext, image: &gltf::image::Data) -> Self {
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

        println!("Creating scratch buffer..");
        let scratch_buffer = create_scratch_buffer(
            device,
            instance,
            physical_device,
            descriptor_pool,
            descriptor_set_layout,
            image,
        );

        let image = Image::new(
            device,
            instance,
            physical_device,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            extent,
        );

        transition_image(
            vulkan_context,
            &image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );

        transfer_image(vulkan_context, scratch_buffer, &image);

        transition_image(
            vulkan_context,
            &image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        Self {}
    }
}

unsafe fn create_scratch_buffer(
    device: &ash::Device,
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    image: &gltf::image::Data,
) -> Buffer<u8> {
    if image.format == gltf::image::Format::R8G8B8A8 {
        return Buffer::new(
            device,
            instance,
            physical_device,
            descriptor_pool,
            descriptor_set_layout,
            &image.pixels,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );
    }

    let mut data = image.pixels.iter();
    let image_buffer = ImageBuffer::from_fn(image.width, image.height, |_, _| {
        let r = data.next().unwrap();
        let g = data.next().unwrap();
        let b = data.next().unwrap();
        image::Rgb([*r, *g, *b])
    });

    let converted_image = DynamicImage::ImageRgb8(image_buffer).into_rgba8();
    let data = converted_image.as_raw();
    return Buffer::new(
        device,
        instance,
        physical_device,
        descriptor_pool,
        descriptor_set_layout,
        &data,
        vk::BufferUsageFlags::TRANSFER_SRC,
    );
}

unsafe fn transfer_image(
    vulkan_context: &VulkanContext,
    scratch_buffer: Buffer<u8>,
    image: &Image,
) {
    vulkan_context.one_time_work(|device, command_buffer| {
        device.cmd_copy_buffer_to_image(
            command_buffer,
            scratch_buffer.buffer,
            image.image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[vk::BufferImageCopy {
                image_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image_extent: image.extent,
                ..Default::default()
            }],
        );
    });
}

unsafe fn transition_image(
    vulkan_context: &VulkanContext,
    image: &Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) {
    vulkan_context.one_time_work(|device, command_buffer| {
        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        let src_access_mask = if new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL {
            vk::AccessFlags::empty()
        } else {
            vk::AccessFlags::TRANSFER_WRITE
        };

        let dst_access_mask = if new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL {
            vk::AccessFlags::TRANSFER_WRITE
        } else {
            vk::AccessFlags::SHADER_READ
        };

        let src_stage_mask = if new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL {
            vk::PipelineStageFlags::TOP_OF_PIPE
        } else {
            vk::PipelineStageFlags::TRANSFER
        };

        let dst_stage_mask = if new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL {
            vk::PipelineStageFlags::TRANSFER
        } else {
            vk::PipelineStageFlags::FRAGMENT_SHADER
        };

        let image_barrier = vk::ImageMemoryBarrier::builder()
            .subresource_range(subresource_range)
            .image(image.image)
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask);

        device.cmd_pipeline_barrier(
            command_buffer,
            src_stage_mask,
            dst_stage_mask,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            std::slice::from_ref(&image_barrier),
        );
    })
}
