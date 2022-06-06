use ash::vk;

pub struct SyncStructures {
    pub present_semaphore: vk::Semaphore,
    pub render_semaphore: vk::Semaphore,
    pub render_fence: vk::Fence,
    pub compute_fence: vk::Fence,
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
            let compute_fence = device
                .create_fence(&vk::FenceCreateInfo::builder(), None)
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
                compute_fence,
            }
        }
    }
}
