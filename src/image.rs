use ash::{vk, Device, Instance};

pub static DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;

pub struct Image {
    image: vk::Image,
    image_view: vk::ImageView,
}

impl Image {
    pub unsafe fn new(
        device: &Device,
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        descriptor_pool: vk::DescriptorPool,
        descriptor_set_layout: vk::DescriptorSetLayout,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        extent: vk::Extent3D,
    ) -> Self {
        let image = device
            .create_image(
                &vk::ImageCreateInfo::builder()
                    .format(format)
                    .usage(usage)
                    .extent(extent),
                None,
            )
            .unwrap();
        let image_view = device
            .create_image_view(&vk::ImageViewCreateInfo::builder().image(image), None)
            .unwrap();
        Self { image, image_view }
    }
}
