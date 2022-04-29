use ash::{vk, Device, Instance};

use crate::memory::allocate_memory;

pub static DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;

pub struct Image {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub device_memory: vk::DeviceMemory,
    pub usage: vk::ImageUsageFlags,
    pub format: vk::Format,
    pub extent: vk::Extent3D,
}

impl Image {
    pub unsafe fn new(
        device: &Device,
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        extent: vk::Extent3D,
    ) -> Self {
        let image = device
            .create_image(
                &vk::ImageCreateInfo::builder()
                    .format(format)
                    .usage(usage)
                    .extent(extent)
                    .mip_levels(1)
                    .array_layers(1)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .tiling(vk::ImageTiling::OPTIMAL),
                None,
            )
            .unwrap();
        let view = device
            .create_image_view(&vk::ImageViewCreateInfo::builder().image(image), None)
            .unwrap();

        let memory_requirements = device.get_image_memory_requirements(image);
        let flags = vk::MemoryPropertyFlags::DEVICE_LOCAL;
        let device_memory = allocate_memory(
            device,
            instance,
            physical_device,
            memory_requirements,
            flags,
        );

        device.bind_image_memory(image, device_memory, 0).unwrap();

        Self {
            image,
            view,
            device_memory,
            usage,
            format,
            extent,
        }
    }
}
