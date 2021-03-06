use crate::{
    frame::Frame,
    image::{Image, DEPTH_FORMAT},
    model::{Material, ModelContext, ModelData},
    swapchain::Swapchain,
    vertex::Vertex,
};
use ash::{
    extensions::{self, khr::Swapchain as SwapchainLoader},
    vk::{self, KhrShaderDrawParametersFn},
};
use nalgebra_glm::{TMat4x4, Vec4};
use std::{
    ffi::{CStr, CString},
    mem::size_of,
};
use vk_shader_macros::include_glsl;
use winit::window::Window;

use crate::buffer::Buffer;

static VERT: &[u32] = include_glsl!("src/shaders/render.vert");
static FRAG: &[u32] = include_glsl!("src/shaders/render.frag");
static COMPUTE: &[u32] = include_glsl!("src/shaders/render.comp");
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

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Globals {
    pub projection: TMat4x4<f32>,
    pub view: TMat4x4<f32>,
    pub camera_position: Vec4,
    pub light_position: Vec4,
}

#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct DrawData {
    pub model_id: u16,
    pub material_id: u16,
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
    pub compute_pipeline: vk::Pipeline,
    pub vertex_buffer: Buffer<Vertex>,
    pub index_buffer: Buffer<u32>,
    pub model_buffer: Buffer<ModelData>,
    pub material_buffer: Buffer<Material>,
    pub draw_data_buffer: Buffer<DrawData>,
    pub shared_descriptor_set: vk::DescriptorSet,
    pub indirect_buffer: Buffer<vk::DrawIndexedIndirectCommand>,
    pub shared_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub pipeline_layout: vk::PipelineLayout,
    pub depth_image: Image,
    pub frames: Vec<Frame>,
    pub frame_index: usize,
    pub sampler: vk::Sampler,
}

impl VulkanContext {
    pub fn new(window: &Window, gpu_type: vk::PhysicalDeviceType) -> Self {
        unsafe {
            let (entry, instance) = init(window);
            let (physical_device, device, queue_family_index) = get_device(&instance, gpu_type);
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
            let (shared_layout, pipeline_layout) = create_descriptor_layouts(&device);

            let shader_stages = create_shader_stages(&device, VERT, FRAG);
            let colored_pipeline = create_pipeline(
                &device,
                &render_pass,
                &swapchain,
                &shader_stages,
                pipeline_layout,
            );
            let compute_pipeline = create_compute_pipeline(&device, pipeline_layout);

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
                &[],
                vk::BufferUsageFlags::VERTEX_BUFFER,
                2097246,
            );

            let index_buffer = Buffer::new(
                &device,
                &instance,
                physical_device,
                &[],
                vk::BufferUsageFlags::INDEX_BUFFER,
                11240796,
            );

            let mut descriptor_counts =
                vk::DescriptorSetVariableDescriptorCountAllocateInfo::builder()
                    .descriptor_counts(&[1000]);

            let shared_descriptor_set = device
                .allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::builder()
                        .descriptor_pool(descriptor_pool)
                        .set_layouts(std::slice::from_ref(&shared_layout))
                        .push_next(&mut descriptor_counts),
                )
                .unwrap()[0];

            let draw_data_buffer = storage_buffer(
                &device,
                &instance,
                physical_device,
                shared_descriptor_set,
                0,
            );

            let model_buffer = storage_buffer(
                &device,
                &instance,
                physical_device,
                shared_descriptor_set,
                1,
            );
            let material_buffer = storage_buffer(
                &device,
                &instance,
                physical_device,
                shared_descriptor_set,
                2,
            );

            let mut indirect_buffer = Buffer::new(
                &device,
                &instance,
                physical_device,
                &[],
                vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::STORAGE_BUFFER
                    | vk::BufferUsageFlags::INDIRECT_BUFFER,
                100_000,
            );
            indirect_buffer.update_descriptor_set(&device, shared_descriptor_set, 3);

            let filter = vk::Filter::LINEAR;
            let address_mode = vk::SamplerAddressMode::REPEAT;
            let sampler = device
                .create_sampler(
                    &vk::SamplerCreateInfo::builder()
                        .mag_filter(filter)
                        .min_filter(filter)
                        .address_mode_u(address_mode)
                        .address_mode_v(address_mode)
                        .address_mode_w(address_mode),
                    None,
                )
                .unwrap();

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
                compute_pipeline,
                present_queue,
                vertex_buffer,
                index_buffer,
                model_buffer,
                material_buffer,
                draw_data_buffer,
                indirect_buffer,
                shared_layout,
                shared_descriptor_set,
                descriptor_pool,
                pipeline_layout,
                depth_image,
                frames,
                frame_index: 0,
                sampler,
            }
        }
    }

    pub unsafe fn render(&mut self, model_context: &ModelContext, globals: &mut Globals) {
        let frame = &self.frames[self.frame_index];
        let sync_structures = &frame.sync_structures;
        let render_fence = &sync_structures.render_fence;
        let present_semaphore = &sync_structures.present_semaphore;
        let render_semaphore = &sync_structures.render_semaphore;
        let device = &self.device;
        let swapchain = &self.swapchain;

        let models = &model_context.models;
        let meshes = &model_context.meshes;
        let draw_commands =
            self.build_draw_commands(models, meshes, model_context, &self.indirect_buffer);

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

        // Run GPU Culling
        self.cull_objects(device, sync_structures, &draw_commands, &globals);

        // Draw the objects!
        self.draw(globals, frame, swapchain_image_index, draw_commands);

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

    unsafe fn draw(
        &self,
        globals: &Globals,
        frame: &Frame,
        swapchain_image_index: u32,
        draw_commands: Vec<vk::DrawIndexedIndirectCommand>,
    ) {
        let device = &self.device;
        let sync_structures = &frame.sync_structures;
        let render_fence = sync_structures.render_fence;
        let present_semaphore = &sync_structures.present_semaphore;
        let render_semaphore = &sync_structures.render_semaphore;
        let command_buffer = frame.command_buffer;

        let swapchain = &self.swapchain;
        let render_pass = self.render_pass;
        let framebuffers = &self.framebuffers;
        let pipeline = &self.colored_pipeline;

        let index_buffer = self.index_buffer.buffer;
        let vertex_buffer = self.vertex_buffer.buffer;
        let indirect_buffer = &self.indirect_buffer;

        let pipeline_layout = self.pipeline_layout;
        let global_push_constant = std::slice::from_raw_parts(
            (globals as *const Globals) as *const u8,
            size_of::<Globals>(),
        );

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
                    float32: [0.2, 0.2, 0.2, 0.0],
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
        device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);
        device.cmd_bind_vertex_buffers(
            command_buffer,
            0,
            std::slice::from_ref(&vertex_buffer),
            &[0],
        );
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline_layout,
            0,
            &[self.shared_descriptor_set],
            &[],
        );
        device.cmd_push_constants(
            command_buffer,
            pipeline_layout,
            vk::ShaderStageFlags::COMPUTE
                | vk::ShaderStageFlags::VERTEX
                | vk::ShaderStageFlags::FRAGMENT,
            0,
            global_push_constant,
        );
        let stride = size_of::<vk::DrawIndexedIndirectCommand>() as _;
        device.cmd_draw_indexed_indirect(
            command_buffer,
            indirect_buffer.buffer,
            0,
            draw_commands.len() as _,
            stride,
        );
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
                render_fence,
            )
            .unwrap();
    }

    unsafe fn build_draw_commands(
        &self,
        models: &Vec<crate::model::Model>,
        meshes: &id_arena::Arena<crate::model::Mesh>,
        model_context: &ModelContext,
        indirect_buffer: &Buffer<vk::DrawIndexedIndirectCommand>,
    ) -> Vec<vk::DrawIndexedIndirectCommand> {
        let mut draw_commands = Vec::new();
        let mut draw_data = Vec::new();
        let mut model_data = Vec::new();
        for (index, model) in models.iter().enumerate() {
            let mesh = meshes.get(model.mesh).unwrap();
            for primitive in &mesh.primitives {
                draw_commands.push(vk::DrawIndexedIndirectCommand {
                    index_count: primitive.num_indices,
                    instance_count: 1,
                    first_index: primitive.index_offset,
                    vertex_offset: primitive.vertex_offset as _,
                    first_instance: 0,
                });

                draw_data.push(DrawData {
                    material_id: primitive.material_id,
                    model_id: index as _,
                })
            }

            model_data.push(model.get_model_data(&mesh));
        }
        // Copy model data into model buffer.
        self.model_buffer.overwrite(&model_data);
        // Upload materials
        self.material_buffer.overwrite(&model_context.materials);
        // Upload draw commands to the GPU.
        indirect_buffer.overwrite(&draw_commands);
        self.draw_data_buffer.overwrite(&draw_data);
        draw_commands
    }

    unsafe fn cull_objects(
        &self,
        device: &ash::Device,
        sync_structures: &crate::sync_structures::SyncStructures,
        draw_commands: &Vec<vk::DrawIndexedIndirectCommand>,
        globals: &Globals,
    ) {
        let compute_command_buffer = create_command_buffer(device, self.command_pool);
        let global_push_constant = std::slice::from_raw_parts(
            (globals as *const Globals) as *const u8,
            size_of::<Globals>(),
        );
        device
            .begin_command_buffer(
                compute_command_buffer,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .unwrap();
        device.cmd_bind_descriptor_sets(
            compute_command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            self.pipeline_layout,
            0,
            &[self.shared_descriptor_set],
            &[],
        );
        device.cmd_push_constants(
            compute_command_buffer,
            self.pipeline_layout,
            vk::ShaderStageFlags::COMPUTE
                | vk::ShaderStageFlags::VERTEX
                | vk::ShaderStageFlags::FRAGMENT,
            0,
            global_push_constant,
        );
        device.cmd_bind_pipeline(
            compute_command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            self.compute_pipeline,
        );
        device.cmd_dispatch(compute_command_buffer, draw_commands.len() as _, 1, 1);
        device.end_command_buffer(compute_command_buffer).unwrap();
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(std::slice::from_ref(&compute_command_buffer));
        device
            .queue_submit(
                self.present_queue,
                std::slice::from_ref(&submit_info),
                sync_structures.compute_fence,
            )
            .unwrap();
        device
            .wait_for_fences(&[sync_structures.compute_fence], true, 1000000000)
            .unwrap();
        device
            .reset_fences(&[sync_structures.compute_fence])
            .unwrap();
        device.free_command_buffers(self.command_pool, &[compute_command_buffer]);
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

unsafe fn storage_buffer<T>(
    device: &ash::Device,
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    descriptor_set: vk::DescriptorSet,
    binding: usize,
) -> Buffer<T> {
    let mut buffer = Buffer::new(
        device,
        instance,
        physical_device,
        &[],
        vk::BufferUsageFlags::STORAGE_BUFFER,
        10_000,
    );
    buffer.update_descriptor_set(device, descriptor_set, binding);
    buffer
}

unsafe fn create_compute_pipeline(
    device: &ash::Device,
    layout: vk::PipelineLayout,
) -> vk::Pipeline {
    let shader_entry_name = CStr::from_bytes_with_nul_unchecked(b"main\0");
    let compute_module = device
        .create_shader_module(&vk::ShaderModuleCreateInfo::builder().code(&COMPUTE), None)
        .unwrap();
    let create_info = vk::ComputePipelineCreateInfo::builder()
        .stage(vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::COMPUTE,
            module: compute_module,
            p_name: shader_entry_name.as_ptr(),
            ..Default::default()
        })
        .layout(layout);

    device
        .create_compute_pipelines(
            vk::PipelineCache::null(),
            std::slice::from_ref(&create_info),
            None,
        )
        .unwrap()[0]
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
    let pool_sizes = [
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::STORAGE_BUFFER,
            descriptor_count: 100,
        },
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1000,
        },
    ];
    device
        .create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&pool_sizes)
                .max_sets(1000)
                .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND),
            None,
        )
        .unwrap()
}

unsafe fn create_descriptor_layouts(
    device: &ash::Device,
) -> (vk::DescriptorSetLayout, vk::PipelineLayout) {
    let bindings = [
        // Draw Data
        vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::COMPUTE,
            descriptor_count: 1,
            ..Default::default()
        },
        // Models
        vk::DescriptorSetLayoutBinding {
            binding: 1,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::COMPUTE,
            descriptor_count: 1,
            ..Default::default()
        },
        // Materials
        vk::DescriptorSetLayoutBinding {
            binding: 2,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            descriptor_count: 1,
            ..Default::default()
        },
        // Draw Calls
        vk::DescriptorSetLayoutBinding {
            binding: 3,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            descriptor_count: 1,
            ..Default::default()
        },
        // Textures
        vk::DescriptorSetLayoutBinding {
            binding: 4,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            descriptor_count: 1000,
            ..Default::default()
        },
    ];

    let flags = vk::DescriptorBindingFlags::PARTIALLY_BOUND
        | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
        | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND;
    let descriptor_flags = [
        vk::DescriptorBindingFlags::empty(),
        vk::DescriptorBindingFlags::empty(),
        vk::DescriptorBindingFlags::empty(),
        vk::DescriptorBindingFlags::empty(),
        flags,
    ];
    let mut binding_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfoEXT::builder()
        .binding_flags(&descriptor_flags);

    let shared_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&bindings)
                .push_next(&mut binding_flags)
                .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL),
            None,
        )
        .unwrap();

    let pipeline_layout = device
        .create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&[shared_layout])
                .push_constant_ranges(&[vk::PushConstantRange {
                    stage_flags: vk::ShaderStageFlags::COMPUTE
                        | vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT,
                    offset: 0,
                    size: size_of::<Globals>() as _,
                    ..Default::default()
                }]),
            None,
        )
        .unwrap();

    (shared_layout, pipeline_layout)
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
    let mut extensions = ash_window::enumerate_required_extensions(window)
        .unwrap()
        .to_vec();
    extensions.push(extensions::khr::GetPhysicalDeviceProperties2::name().as_ptr());
    let instance = entry
        .create_instance(
            &vk::InstanceCreateInfo::builder()
                .enabled_extension_names(&extensions)
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

unsafe fn get_device(
    instance: &ash::Instance,
    gpu_type: vk::PhysicalDeviceType,
) -> (vk::PhysicalDevice, ash::Device, u32) {
    let (physical_device, queue_index) = instance
        .enumerate_physical_devices()
        .unwrap()
        .drain(..)
        .find_map(|physical_device| {
            let physical_properties = instance.get_physical_device_properties(physical_device);
            if physical_properties.device_type != gpu_type {
                return None;
            }
            instance
                .get_physical_device_queue_family_properties(physical_device)
                .iter()
                .enumerate()
                .find_map(|(index, info)| {
                    if info.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                        println!(
                            "Using device {:?}",
                            ::std::ffi::CStr::from_ptr(physical_properties.device_name.as_ptr())
                        );
                        Some((physical_device, index as _))
                    } else {
                        None
                    }
                })
        })
        .unwrap();

    let device_extension_names = [
        SwapchainLoader::name().as_ptr(),
        KhrShaderDrawParametersFn::name().as_ptr(),
    ];
    let queue_create_info = vk::DeviceQueueCreateInfo::builder()
        .queue_priorities(&[1.0])
        .queue_family_index(queue_index);

    let mut vulkan_11_features = vk::PhysicalDeviceVulkan11Features::builder()
        .shader_draw_parameters(true)
        .storage_buffer16_bit_access(true);

    let enabled_features = vk::PhysicalDeviceFeatures::builder()
        .multi_draw_indirect(true)
        .shader_int16(true);

    let mut descriptor_indexing_features = vk::PhysicalDeviceDescriptorIndexingFeatures::builder()
        .shader_sampled_image_array_non_uniform_indexing(true)
        .descriptor_binding_partially_bound(true)
        .descriptor_binding_variable_descriptor_count(true)
        .descriptor_binding_sampled_image_update_after_bind(true)
        .runtime_descriptor_array(true);

    let mut robust_features =
        vk::PhysicalDeviceRobustness2FeaturesEXT::builder().null_descriptor(true);

    let device_create_info = vk::DeviceCreateInfo::builder()
        .enabled_extension_names(&device_extension_names)
        .queue_create_infos(std::slice::from_ref(&queue_create_info))
        .enabled_features(&enabled_features)
        .push_next(&mut vulkan_11_features)
        .push_next(&mut robust_features)
        .push_next(&mut descriptor_indexing_features);

    let device = instance
        .create_device(physical_device, &device_create_info, None)
        .unwrap();

    (physical_device, device, queue_index)
}
