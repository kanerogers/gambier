use crate::{
    frame::Frame,
    image::{Image, DEPTH_FORMAT},
    model::{Mesh, Model, Vertex},
    swapchain::Swapchain,
};
use ash::{extensions::khr::Swapchain as SwapchainLoader, vk};
use id_arena::Arena;
use nalgebra_glm::TMat4x4;
use std::ffi::{CStr, CString};
use vk_shader_macros::include_glsl;
use winit::window::Window;

use crate::buffer::Buffer;

static COLORED_VERT: &[u32] = include_glsl!("src/shaders/colored_triangle.vert");
static COLORED_FRAG: &[u32] = include_glsl!("src/shaders/colored_triangle.frag");
pub static SWAPCHAIN_LENGTH: u32 = 3;

#[derive(Clone)]
pub enum SelectedPipeline {
    Colored,
}

impl Default for SelectedPipeline {
    fn default() -> Self {
        SelectedPipeline::Colored
    }
}

#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct Globals {
    pub projection: TMat4x4<f32>,
    pub view: TMat4x4<f32>,
    pub model: TMat4x4<f32>,
}

pub struct VulkanContext {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub swapchain: Swapchain,
    pub command_pool: vk::CommandPool,
    pub work_command_buffer: vk::CommandBuffer,
    pub work_fence: vk::Fence,
    pub render_pass: vk::RenderPass,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_image_views: Vec<vk::ImageView>,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub present_queue: vk::Queue,
    pub colored_pipeline: vk::Pipeline,
    pub vertex_buffer: Buffer<Vertex>,
    pub index_buffer: Buffer<u32>,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub pipeline_layout: vk::PipelineLayout,
    pub depth_image: Image,
    pub frames: Vec<Frame>,
    pub frame_index: usize,
}

impl VulkanContext {
    pub fn new(window: &Window) -> Self {
        unsafe {
            let (entry, instance) = init(window);
            let (physical_device, device, queue_family_index) = get_device(&instance);
            let present_queue = device.get_device_queue(queue_family_index, 0);
            let swapchain = Swapchain::new(&entry, &instance, window, physical_device, &device);
            let (swapchain_images, swapchain_image_views) = swapchain.create_image_views(&device);
            let depth_extent = vk::Extent3D {
                width: swapchain.resolution.width,
                height: swapchain.resolution.height,
                depth: 1,
            };
            let depth_image = Image::new(
                &device,
                &instance,
                physical_device,
                DEPTH_FORMAT,
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                depth_extent,
            );

            let command_pool = create_command_pool(&device, queue_family_index);
            let work_command_buffer = create_command_buffer(&device, command_pool);
            let work_fence = device
                .create_fence(&vk::FenceCreateInfo::builder(), None)
                .unwrap();
            let frames = (0..3).map(|_| Frame::new(&device, command_pool)).collect();
            let render_pass = create_render_pass(&device, &swapchain);
            let (descriptor_set_layout, pipeline_layout) = create_descriptor_layouts(&device);

            let shader_stages = create_shader_stages(&device, COLORED_VERT, COLORED_FRAG);
            let colored_pipeline = create_pipeline(
                &device,
                &render_pass,
                &swapchain,
                &shader_stages,
                pipeline_layout,
            );

            // Resources
            let framebuffers = create_framebuffers(
                &device,
                &swapchain,
                &swapchain_image_views,
                depth_image.view,
                &render_pass,
            );
            let descriptor_pool = create_descriptor_pool(&device);
            let vertex_buffer = Buffer::new(
                &device,
                &instance,
                physical_device,
                descriptor_pool,
                descriptor_set_layout,
                &[],
                vk::BufferUsageFlags::VERTEX_BUFFER,
            );

            let index_buffer = Buffer::new(
                &device,
                &instance,
                physical_device,
                descriptor_pool,
                descriptor_set_layout,
                &[],
                vk::BufferUsageFlags::INDEX_BUFFER,
            );

            Self {
                entry,
                instance,
                physical_device,
                device,
                swapchain,
                command_pool,
                work_command_buffer,
                work_fence,
                render_pass,
                swapchain_images,
                swapchain_image_views,
                framebuffers,
                colored_pipeline,
                present_queue,
                vertex_buffer,
                index_buffer,
                descriptor_set_layout,
                descriptor_pool,
                pipeline_layout,
                depth_image,
                frames,
                frame_index: 0,
            }
        }
    }

    pub unsafe fn render(&mut self, models: &[Model], meshes: &Arena<Mesh>, globals: &mut Globals) {
        let frame = &self.frames[self.frame_index];
        let sync_structures = &frame.sync_structures;
        let render_fence = &sync_structures.render_fence;
        let present_semaphore = &sync_structures.present_semaphore;
        let render_semaphore = &sync_structures.render_semaphore;
        let command_buffer = frame.command_buffer;

        let device = &self.device;
        let swapchain = &self.swapchain;
        let render_pass = self.render_pass;
        let framebuffers = &self.framebuffers;
        let pipeline = &self.colored_pipeline;

        let index_buffer = self.index_buffer.buffer;
        let vertex_buffer = self.vertex_buffer.buffer;

        let pipeline_layout = self.pipeline_layout;
        let _descriptor_sets = [self.vertex_buffer.descriptor_set];

        let global_push_constant = std::slice::from_raw_parts(
            (globals as *const Globals) as *const u8,
            std::mem::size_of::<Globals>(),
        );

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
            .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
            .unwrap();
        device
            .begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .unwrap();

        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [1.0, 1.0, 1.0, 0.0],
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
            .render_pass(render_pass)
            .framebuffer(framebuffers[swapchain_image_index as usize])
            .render_area(swapchain.resolution.into())
            .clear_values(&clear_values);
        device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_begin_info,
            vk::SubpassContents::INLINE,
        );

        device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, *pipeline);
        // device.cmd_bind_descriptor_sets(
        //     *command_buffer,
        //     vk::PipelineBindPoint::GRAPHICS,
        //     pipeline_layout,
        //     0,
        //     &descriptor_sets,
        //     &[],
        // );
        device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);
        device.cmd_bind_vertex_buffers(
            command_buffer,
            0,
            std::slice::from_ref(&vertex_buffer),
            &[0],
        );

        for model in models {
            let mesh = meshes.get(model.mesh).unwrap();
            for primitive in &mesh.primitives {
                globals.model = model.transform;
                device.cmd_push_constants(
                    command_buffer,
                    pipeline_layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    global_push_constant,
                );
                device.cmd_draw_indexed(
                    command_buffer,
                    primitive.num_indices,
                    1,
                    primitive.index_offset,
                    primitive.vertex_offset as _,
                    0,
                );
            }
        }

        device.cmd_end_render_pass(command_buffer);
        device.end_command_buffer(command_buffer).unwrap();

        // Submit
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(std::slice::from_ref(&command_buffer))
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

        self.frame_index = (self.frame_index + 1) % 3;
    }

    pub unsafe fn one_time_work<F>(&self, work: F) -> ()
    where
        F: FnOnce(&ash::Device, vk::CommandBuffer),
    {
        let device = &self.device;
        let command_buffer = self.work_command_buffer;
        let fence = self.work_fence;

        device
            .begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .unwrap();

        work(device, command_buffer);
        device.end_command_buffer(command_buffer).unwrap();

        let submit_info =
            vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&command_buffer));
        device
            .queue_submit(
                self.present_queue,
                std::slice::from_ref(&submit_info),
                fence,
            )
            .unwrap();

        device
            .wait_for_fences(std::slice::from_ref(&fence), true, 1_000_000_000)
            .unwrap();
        device.reset_fences(std::slice::from_ref(&fence)).unwrap();
    }
}

pub unsafe fn create_command_buffer(
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

unsafe fn create_descriptor_pool(device: &ash::Device) -> vk::DescriptorPool {
    let pool_sizes = [vk::DescriptorPoolSize {
        ty: vk::DescriptorType::STORAGE_BUFFER,
        descriptor_count: 1,
    }];
    device
        .create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&pool_sizes)
                .max_sets(4),
            None,
        )
        .unwrap()
}

unsafe fn create_descriptor_layouts(
    device: &ash::Device,
) -> (vk::DescriptorSetLayout, vk::PipelineLayout) {
    let bindings = [vk::DescriptorSetLayoutBinding {
        binding: 0,
        descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
        stage_flags: vk::ShaderStageFlags::VERTEX,
        descriptor_count: 1,
        ..Default::default()
    }];

    let descriptor_set_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings),
            None,
        )
        .unwrap();

    let pipeline_layout = device
        .create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&[descriptor_set_layout])
                .push_constant_ranges(&[vk::PushConstantRange {
                    stage_flags: vk::ShaderStageFlags::VERTEX,
                    offset: 0,
                    size: std::mem::size_of::<Globals>() as _,
                    ..Default::default()
                }]),
            None,
        )
        .unwrap();

    (descriptor_set_layout, pipeline_layout)
}

unsafe fn create_shader_stages(
    device: &ash::Device,
    vertex_shader: &[u32],
    fragment_shader: &[u32],
) -> [vk::PipelineShaderStageCreateInfo; 2] {
    let shader_entry_name = CStr::from_bytes_with_nul_unchecked(b"main\0");
    let vertex_module = device
        .create_shader_module(
            &vk::ShaderModuleCreateInfo::builder().code(vertex_shader),
            None,
        )
        .unwrap();
    let fragment_module = device
        .create_shader_module(
            &vk::ShaderModuleCreateInfo::builder().code(fragment_shader),
            None,
        )
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
    shader_stages
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
                        .application_name(&CString::new("Gambier Test").unwrap()),
                ),
            None,
        )
        .unwrap();
    (entry, instance)
}

unsafe fn create_pipeline(
    device: &ash::Device,
    render_pass: &vk::RenderPass,
    swapchain: &Swapchain,
    shader_stages: &[vk::PipelineShaderStageCreateInfo],
    pipeline_layout: vk::PipelineLayout,
) -> vk::Pipeline {
    let vertex_input_description = Vertex::description();
    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_attribute_descriptions(&vertex_input_description.attributes)
        .vertex_binding_descriptions(&vertex_input_description.bindings);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE);

    let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1)
        .min_sample_shading(1.)
        .alpha_to_coverage_enable(false)
        .alpha_to_one_enable(false);

    // TODO: Revisit.
    let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
        depth_test_enable: 1,
        depth_write_enable: 1,
        depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
        depth_bounds_test_enable: 0,
        min_depth_bounds: 0.,
        max_depth_bounds: 1.,
        stencil_test_enable: 0,
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
        .max_depth(1.);

    let scissor = swapchain.resolution.into();

    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(std::slice::from_ref(&viewport))
        .scissors(std::slice::from_ref(&scissor));

    // let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    // let dynamic_state =
    //     &vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

    // TODO: Understand depth stencil state better - I always muck this up.
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
    depth_image_view: vk::ImageView,
    render_pass: &vk::RenderPass,
) -> Vec<vk::Framebuffer> {
    swapchain_image_views
        .iter()
        .map(|image_view| {
            let attachments = [*image_view, depth_image_view];
            let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(*render_pass)
                .layers(1)
                .width(swapchain.resolution.width)
                .height(swapchain.resolution.height)
                .attachments(&attachments);

            unsafe {
                device
                    .create_framebuffer(&framebuffer_create_info, None)
                    .unwrap()
            }
        })
        .collect()
}

unsafe fn create_render_pass(device: &ash::Device, swapchain: &Swapchain) -> vk::RenderPass {
    let attachments = [
        vk::AttachmentDescription {
            format: swapchain.format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        },
        vk::AttachmentDescription {
            format: DEPTH_FORMAT,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            ..Default::default()
        },
    ];
    let color_attachment_refs = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];
    let depth_attachment_ref = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };

    let colour_dependency = vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        dst_subpass: 0,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        src_access_mask: vk::AccessFlags::empty(),
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        ..Default::default()
    };

    let depth_dependency = vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        dst_subpass: 0,
        src_stage_mask: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
            | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
        src_access_mask: vk::AccessFlags::empty(),
        dst_stage_mask: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
            | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
        dst_access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        ..Default::default()
    };

    let dependencies = [colour_dependency, depth_dependency];

    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachment_refs)
        .depth_stencil_attachment(&depth_attachment_ref);

    let create_info = &vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(std::slice::from_ref(&subpass))
        .dependencies(&dependencies);

    device.create_render_pass(&create_info, None).unwrap()
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
