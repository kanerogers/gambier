use std::ffi::CString;

use ash::{
    extensions::khr::{Surface as SurfaceLoader, Swapchain as SwapchainLoader},
    vk::{self, SurfaceKHR},
};
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub struct Swapchain {
    pub loader: SwapchainLoader,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_image_views: Vec<vk::ImageView>,
}

impl Swapchain {
    unsafe fn new(
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
        let surface_format = surface_loader
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
                    .image_format(surface_format)
                    .surface(surface)
                    .min_image_count(3)
                    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT),
                None,
            )
            .unwrap();

        let swapchain_images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
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
                            .format(surface_format),
                        None,
                    )
                    .unwrap()
            })
            .collect();

        Self {
            loader: swapchain_loader,
            swapchain,
            swapchain_images,
            swapchain_image_views,
        }
    }
}

pub struct VulkanContext {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub swapchain: Swapchain,
}

impl VulkanContext {
    pub fn new(window: &Window) -> Self {
        let (entry, instance) = unsafe { init(window) };
        let (physical_device, device) = unsafe { get_device(&instance) };
        let swapchain =
            unsafe { Swapchain::new(&entry, &instance, window, physical_device, &device) };
        Self {
            entry,
            instance,
            physical_device,
            device,
            swapchain,
        }
    }
}

unsafe fn get_device(instance: &ash::Instance) -> (vk::PhysicalDevice, ash::Device) {
    let (physical_device, queue_index) = instance
        .enumerate_physical_devices()
        .unwrap()
        .drain(..)
        .find_map(|physical_device| {
            let physical_properties = instance.get_physical_device_properties(physical_device);
            if physical_properties.device_type != vk::PhysicalDeviceType::DISCRETE_GPU {
                return None;
            }
            instance
                .get_physical_device_queue_family_properties(physical_device)
                .iter()
                .enumerate()
                .find_map(|(index, info)| {
                    if info.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                        Some((physical_device, index))
                    } else {
                        None
                    }
                })
        })
        .unwrap();

    let device_extension_names = [SwapchainLoader::name().as_ptr()];
    let queue_create_info = vk::DeviceQueueCreateInfo::builder()
        .queue_priorities(&[1.0])
        .queue_family_index(queue_index as _);

    let device = instance
        .create_device(
            physical_device,
            &vk::DeviceCreateInfo::builder()
                .enabled_extension_names(&device_extension_names)
                .queue_create_infos(std::slice::from_ref(&queue_create_info)),
            None,
        )
        .unwrap();

    (physical_device, device)
}

unsafe fn init(window: &Window) -> (ash::Entry, ash::Instance) {
    let entry = ash::Entry::load().unwrap();
    let extensions = ash_window::enumerate_required_extensions(window).unwrap();
    let instance = entry
        .create_instance(
            &vk::InstanceCreateInfo::builder()
                .enabled_extension_names(extensions)
                .application_info(
                    &vk::ApplicationInfo::builder()
                        .api_version(vk::make_api_version(0, 1, 3, 0))
                        .engine_name(&CString::new("Gambier").unwrap())
                        .application_name(&CString::new("VKHex").unwrap()),
                ),
            None,
        )
        .unwrap();
    (entry, instance)
}

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let context = VulkanContext::new(&window);
}
