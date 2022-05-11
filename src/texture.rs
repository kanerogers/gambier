use ash::vk;
use image::EncodableLayout;

use crate::{buffer::Buffer, image::Image, vulkan_context::VulkanContext};

#[derive(Debug)]
pub struct Texture {
    pub descriptor_set: vk::DescriptorSet,
}

impl Texture {
    pub unsafe fn new(
        vulkan_context: &VulkanContext,
        scratch_buffer: &Buffer<u8>,
        image: image::DynamicImage,
    ) -> Self {
        println!("Creating texture..");
        let device = &vulkan_context.device;
        let instance = &vulkan_context.instance;
        let physical_device = vulkan_context.physical_device;
        let descriptor_pool = vulkan_context.descriptor_pool;
        let descriptor_set_layout = vulkan_context.texture_layout;
        let extent = vk::Extent3D {
            width: image.width(),
            height: image.height(),
            depth: 1,
        };

        let image_data = image.into_rgba8();
        let image_data = image_data.as_bytes();
        scratch_buffer.overwrite(image_data);

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

        let descriptor_set = device
            .allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(descriptor_pool)
                    .set_layouts(&[descriptor_set_layout]),
            )
            .unwrap()[0];

        let image_info = vk::DescriptorImageInfo::builder()
            .sampler(vulkan_context.sampler)
            .image_view(image.view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let write = vk::WriteDescriptorSet::builder()
            .image_info(std::slice::from_ref(&image_info))
            .dst_set(descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER);

        device.update_descriptor_sets(std::slice::from_ref(&write), &[]);

        Self { descriptor_set }
    }
}

pub unsafe fn create_scratch_buffer(vulkan_context: &VulkanContext, size: usize) -> Buffer<u8> {
    return Buffer::new(
        &vulkan_context.device,
        &vulkan_context.instance,
        vulkan_context.physical_device,
        &[],
        vk::BufferUsageFlags::TRANSFER_SRC,
        size,
    );
}

unsafe fn transfer_image(
    vulkan_context: &VulkanContext,
    scratch_buffer: &Buffer<u8>,
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
