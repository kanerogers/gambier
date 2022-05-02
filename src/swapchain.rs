use crate::vulkan_context::SWAPCHAIN_LENGTH;
use ash::{
    extensions::khr::{Surface as SurfaceLoader, Swapchain as SwapchainLoader},
    vk,
};
use winit::window::Window;

pub struct Swapchain {
    pub loader: SwapchainLoader,
    pub swapchain: vk::SwapchainKHR,
    pub format: vk::Format,
    pub resolution: vk::Extent2D,
}

impl Swapchain {
    pub unsafe fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: &Window,
        physical_device: vk::PhysicalDevice,
        device: &ash::Device,
    ) -> Self {
        let surface_loader = SurfaceLoader::new(entry, instance);
        let surface = ash_window::create_surface(entry, instance, window, None).unwrap();

        let surface_capabilities = surface_loader
            .get_physical_device_surface_capabilities(physical_device, surface)
            .unwrap();
        let format = surface_loader
            .get_physical_device_surface_formats(physical_device, surface)
            .unwrap()[0]
            .format;

        let swapchain_loader = SwapchainLoader::new(instance, device);
        let swapchain = swapchain_loader
            .create_swapchain(
                &vk::SwapchainCreateInfoKHR::builder()
                    .image_array_layers(1)
                    .image_extent(surface_capabilities.current_extent)
                    .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
                    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .image_format(format)
                    .surface(surface)
                    .min_image_count(SWAPCHAIN_LENGTH)
                    .present_mode(vk::PresentModeKHR::FIFO)
                    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT),
                None,
            )
            .unwrap();

        Self {
            loader: swapchain_loader,
            swapchain,
            format,
            resolution: surface_capabilities.current_extent,
        }
    }

    pub unsafe fn create_image_views(
        &self,
        device: &ash::Device,
    ) -> (Vec<vk::Image>, Vec<vk::ImageView>) {
        let swapchain = &self;
        let swapchain_images = swapchain
            .loader
            .get_swapchain_images(swapchain.swapchain)
            .unwrap();
        let swapchain_image_views = swapchain_images
            .iter()
            .map(|i| {
                device
                    .create_image_view(
                        &vk::ImageViewCreateInfo::builder()
                            .view_type(vk::ImageViewType::TYPE_2D)
                            .components(vk::ComponentMapping {
                                r: vk::ComponentSwizzle::R,
                                g: vk::ComponentSwizzle::G,
                                b: vk::ComponentSwizzle::B,
                                a: vk::ComponentSwizzle::A,
                            })
                            .subresource_range(vk::ImageSubresourceRange {
                                aspect_mask: vk::ImageAspectFlags::COLOR,
                                base_mip_level: 0,
                                level_count: 1,
                                base_array_layer: 0,
                                layer_count: 1,
                            })
                            .image(*i)
                            .format(swapchain.format),
                        None,
                    )
                    .unwrap()
            })
            .collect();

        (swapchain_images, swapchain_image_views)
    }
}
