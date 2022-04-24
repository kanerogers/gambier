use ash::{
    extensions::khr::{Surface as SurfaceLoader, Swapchain as SwapchainLoader},
    vk::{self},
};
use std::ffi::{CStr, CString};
use vk_shader_macros::include_glsl;
use winit::window::Window;

const VERT: &[u32] = include_glsl!("src/shaders/triangle.vert");
const FRAG: &[u32] = include_glsl!("src/shaders/triangle.frag");

pub struct VulkanContext {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub swapchain: Swapchain,
    pub command_pool: vk::CommandPool,
    pub command_buffer: vk::CommandBuffer,
    pub render_pass: vk::RenderPass,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_image_views: Vec<vk::ImageView>,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub sync_structures: SyncStructures,
    pub pipeline: vk::Pipeline,
    pub present_queue: vk::Queue,
}

impl VulkanContext {
    pub fn new(window: &Window) -> Self {
        unsafe {
            let (entry, instance) = init(window);
            let (physical_device, device, queue_family_index) = get_device(&instance);
            let present_queue = device.get_device_queue(queue_family_index, 0);
            let swapchain = Swapchain::new(&entry, &instance, window, physical_device, &device);
            let (swapchain_images, swapchain_image_views) =
                create_swapchain_image_views(&device, &swapchain);

            let command_pool = create_command_pool(&device, queue_family_index);
            let command_buffer = create_command_buffer(&device, command_pool);
            let render_pass = create_render_pass(&device, &swapchain);
            let framebuffers =
                create_framebuffers(&device, &swapchain, &swapchain_image_views, &render_pass);
            let sync_structures = SyncStructures::new(&device);
            let pipeline = create_pipeline(&device, &render_pass, &swapchain);

            Self {
                entry,
                instance,
                physical_device,
                device,
                swapchain,
                command_pool,
                command_buffer,
                render_pass,
                swapchain_images,
                swapchain_image_views,
                framebuffers,
                sync_structures,
                pipeline,
                present_queue,
            }
        }
    }

    pub unsafe fn render(&self) {
        let render_fence = &self.sync_structures.render_fence;
        let present_semaphore = &self.sync_structures.present_semaphore;
        let render_semaphore = &self.sync_structures.render_semaphore;
        let command_buffer = &self.command_buffer;
        let device = &self.device;
        let swapchain = &self.swapchain;
        let render_pass = &self.render_pass;
        let framebuffers = &self.framebuffers;
        let pipeline = &self.pipeline;

        device
            .wait_for_fences(std::slice::from_ref(render_fence), true, 1000000000)
            .unwrap();
        device
            .reset_fences(std::slice::from_ref(render_fence))
            .unwrap();
        let (swapchain_image_index, _) = swapchain
            .loader
            .acquire_next_image(
                swapchain.swapchain,
                1000000000,
                *present_semaphore,
                vk::Fence::null(),
            )
            .unwrap();
        device
            .reset_command_buffer(*command_buffer, vk::CommandBufferResetFlags::empty())
            .unwrap();
        device
            .begin_command_buffer(
                *command_buffer,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .unwrap();
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(*render_pass)
            .framebuffer(framebuffers[swapchain_image_index as usize])
            .render_area(swapchain.resolution.into())
            .clear_values(&clear_values);
        device.cmd_begin_render_pass(
            *command_buffer,
            &render_pass_begin_info,
            vk::SubpassContents::INLINE,
        );
        device.cmd_bind_pipeline(*command_buffer, vk::PipelineBindPoint::GRAPHICS, *pipeline);
        device.cmd_draw(*command_buffer, 3, 1, 0, 0);
        device.cmd_end_render_pass(*command_buffer);
        device.end_command_buffer(*command_buffer).unwrap();
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(std::slice::from_ref(command_buffer))
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .wait_semaphores(std::slice::from_ref(present_semaphore))
            .signal_semaphores(std::slice::from_ref(render_semaphore));
        device
            .queue_submit(
                self.present_queue,
                std::slice::from_ref(&submit_info),
                *render_fence,
            )
            .unwrap();
        let present_info = vk::PresentInfoKHR::builder()
            .swapchains(std::slice::from_ref(&swapchain.swapchain))
            .wait_semaphores(std::slice::from_ref(render_semaphore))
            .image_indices(std::slice::from_ref(&swapchain_image_index));

        swapchain
            .loader
            .queue_present(self.present_queue, &present_info)
            .unwrap();
    }
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

pub struct Swapchain {
    pub loader: SwapchainLoader,
    pub swapchain: vk::SwapchainKHR,
    pub format: vk::Format,
    pub resolution: vk::Extent2D,
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
                    .min_image_count(3)
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
}

pub struct SyncStructures {
    pub present_semaphore: vk::Semaphore,
    pub render_semaphore: vk::Semaphore,
    pub render_fence: vk::Fence,
}

impl SyncStructures {
    pub fn new(device: &ash::Device) -> Self {
        unsafe {
            let render_fence = device
                .create_fence(
                    &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )
                .unwrap();
            let present_semaphore = device
                .create_semaphore(&vk::SemaphoreCreateInfo::builder(), None)
                .unwrap();
            let render_semaphore = device
                .create_semaphore(&vk::SemaphoreCreateInfo::builder(), None)
                .unwrap();

            Self {
                present_semaphore,
                render_semaphore,
                render_fence,
            }
        }
    }
}

unsafe fn create_pipeline(
    device: &ash::Device,
    render_pass: &vk::RenderPass,
    swapchain: &Swapchain,
) -> vk::Pipeline {
    let shader_entry_name = CStr::from_bytes_with_nul_unchecked(b"main\0");
    let vertex_module = device
        .create_shader_module(&vk::ShaderModuleCreateInfo::builder().code(VERT), None)
        .unwrap();

    let fragment_module = device
        .create_shader_module(&vk::ShaderModuleCreateInfo::builder().code(FRAG), None)
        .unwrap();

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo {
            module: vertex_module,
            p_name: shader_entry_name.as_ptr(),
            stage: vk::ShaderStageFlags::VERTEX,
            ..Default::default()
        },
        vk::PipelineShaderStageCreateInfo {
            module: fragment_module,
            p_name: shader_entry_name.as_ptr(),
            stage: vk::ShaderStageFlags::FRAGMENT,
            ..Default::default()
        },
    ];

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder();
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::CLOCKWISE)
        .depth_bias_enable(false)
        .depth_bias_constant_factor(0.)
        .depth_bias_clamp(0.)
        .depth_bias_slope_factor(0.);

    let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1)
        .min_sample_shading(1.)
        .alpha_to_coverage_enable(false)
        .alpha_to_one_enable(false);

    let noop_stencil_state = vk::StencilOpState {
        fail_op: vk::StencilOp::KEEP,
        pass_op: vk::StencilOp::KEEP,
        depth_fail_op: vk::StencilOp::KEEP,
        compare_op: vk::CompareOp::ALWAYS,
        ..Default::default()
    };
    let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
        depth_test_enable: 1,
        depth_write_enable: 1,
        depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
        front: noop_stencil_state,
        back: noop_stencil_state,
        max_depth_bounds: 1.0,
        ..Default::default()
    };

    let color_blend_attachment_state = vk::PipelineColorBlendAttachmentState::builder()
        .blend_enable(false)
        .color_write_mask(
            vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
        );

    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .attachments(std::slice::from_ref(&color_blend_attachment_state))
        .logic_op_enable(false)
        .logic_op(vk::LogicOp::COPY);

    let viewport = vk::Viewport::builder()
        .x(0.)
        .y(0.)
        .height(swapchain.resolution.height as _)
        .width(swapchain.resolution.width as _)
        .min_depth(0.)
        .max_depth(0.);

    let scissor = swapchain.resolution.into();

    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(std::slice::from_ref(&viewport))
        .scissors(std::slice::from_ref(&scissor));

    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state =
        &vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

    let pipeline_layout = device
        .create_pipeline_layout(&Default::default(), None)
        .unwrap();

    let create_infos = vk::GraphicsPipelineCreateInfo::builder()
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .color_blend_state(&color_blend_state)
        .depth_stencil_state(&depth_state_info)
        .render_pass(*render_pass)
        .layout(pipeline_layout)
        .stages(&shader_stages);

    device
        .create_graphics_pipelines(
            vk::PipelineCache::null(),
            std::slice::from_ref(&create_infos),
            None,
        )
        .unwrap()[0]
}

fn create_framebuffers(
    device: &ash::Device,
    swapchain: &Swapchain,
    swapchain_image_views: &[vk::ImageView],
    render_pass: &vk::RenderPass,
) -> Vec<vk::Framebuffer> {
    swapchain_image_views
        .iter()
        .map(|image_view| {
            let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(*render_pass)
                .layers(1)
                .width(swapchain.resolution.width)
                .height(swapchain.resolution.height)
                .attachments(std::slice::from_ref(image_view));
            unsafe {
                device
                    .create_framebuffer(&framebuffer_create_info, None)
                    .unwrap()
            }
        })
        .collect()
}

unsafe fn create_swapchain_image_views(
    device: &ash::Device,
    swapchain: &Swapchain,
) -> (Vec<vk::Image>, Vec<vk::ImageView>) {
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

unsafe fn create_render_pass(device: &ash::Device, swapchain: &Swapchain) -> vk::RenderPass {
    let color_attachment = vk::AttachmentDescription::builder()
        .format(swapchain.format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    let color_attachment_ref = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(std::slice::from_ref(&color_attachment_ref));

    device
        .create_render_pass(
            &vk::RenderPassCreateInfo::builder()
                .attachments(std::slice::from_ref(&color_attachment))
                .subpasses(std::slice::from_ref(&subpass)),
            None,
        )
        .unwrap()
}

unsafe fn create_command_buffer(
    device: &ash::Device,
    command_pool: vk::CommandPool,
) -> vk::CommandBuffer {
    device
        .allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::builder()
                .command_buffer_count(1)
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY),
        )
        .unwrap()[0]
}

unsafe fn create_command_pool(device: &ash::Device, queue_family_index: u32) -> vk::CommandPool {
    device
        .create_command_pool(
            &vk::CommandPoolCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
            None,
        )
        .unwrap()
}

unsafe fn get_device(instance: &ash::Instance) -> (vk::PhysicalDevice, ash::Device, u32) {
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
                        Some((physical_device, index as _))
                    } else {
                        None
                    }
                })
        })
        .unwrap();

    let device_extension_names = [SwapchainLoader::name().as_ptr()];
    let queue_create_info = vk::DeviceQueueCreateInfo::builder()
        .queue_priorities(&[1.0])
        .queue_family_index(queue_index);

    let device = instance
        .create_device(
            physical_device,
            &vk::DeviceCreateInfo::builder()
                .enabled_extension_names(&device_extension_names)
                .queue_create_infos(std::slice::from_ref(&queue_create_info)),
            None,
        )
        .unwrap();

    (physical_device, device, queue_index)
}
