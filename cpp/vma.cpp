#define VMA_IMPLEMENTATION
#include "./vma.h"

typedef void *VMA_ALLOCATOR_HANDLE;
typedef void *VMA_MEMORY_HANDLE;

#ifdef _WIN32
#define VMA_EXPORT __declspec(dllexport)
#else
#define VMA_EXPORT
#endif

extern "C" VMA_EXPORT VkResult create_allocator(uint64_t instance, uint64_t physical_device, uint64_t device, VMA_ALLOCATOR_HANDLE *allocator)
{
    VmaAllocatorCreateInfo allocatorInfo = {};
    allocatorInfo.device = reinterpret_cast<VkDevice>(device);
    allocatorInfo.physicalDevice = reinterpret_cast<VkPhysicalDevice>(physical_device);
    allocatorInfo.instance = reinterpret_cast<VkInstance>(instance);

    VmaAllocator a;
    VkResult result = vmaCreateAllocator(&allocatorInfo, &a);

    if (result != VK_SUCCESS)
    {
        return result;
    }

    *allocator = reinterpret_cast<VMA_ALLOCATOR_HANDLE>(a);
    return result;
}

extern "C" VMA_EXPORT VkResult allocate_memory_for_buffer(VMA_ALLOCATOR_HANDLE allocator, uint64_t buffer_object, bool host_visible, VMA_MEMORY_HANDLE* allocation)
{
    auto vma_allocator = reinterpret_cast<VmaAllocator>(allocator);
    VmaAllocationCreateInfo alloc_info = {};
    alloc_info.requiredFlags =
        (host_visible
             ? (VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT)
             : VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT);

    VmaAllocation a;
    VmaAllocationInfo a_info;
    VkResult result = vmaAllocateMemoryForBuffer(vma_allocator, reinterpret_cast<VkBuffer>(buffer_object), &alloc_info, &a, &a_info);

    if (result != VK_SUCCESS)
    {
        return result;
    }

    result = vmaBindBufferMemory(vma_allocator, a, reinterpret_cast<VkBuffer>(buffer_object));
    if (result != VK_SUCCESS) {
        vmaFreeMemory(vma_allocator, a);
        return result;
    }

    *allocation = reinterpret_cast<VMA_MEMORY_HANDLE>(a);
    return result;
}

extern "C" VMA_EXPORT VkResult set_memory_data(VMA_ALLOCATOR_HANDLE allocator, VMA_MEMORY_HANDLE memory, const void* data_in, size_t size) {
    auto vma_allocator = reinterpret_cast<VmaAllocator>(allocator);
    auto vma_allocation = reinterpret_cast<VmaAllocation>(memory);

    void* data = nullptr;
    VkResult result = vmaMapMemory(vma_allocator, vma_allocation, &data);
    if (result != VK_SUCCESS || data == nullptr) {
        return result;
    }

    // UNSAFE
    memcpy(data, data_in, size);

    vmaUnmapMemory(vma_allocator, vma_allocation);

    return VK_SUCCESS;
}

extern "C" VMA_EXPORT void free_memory(VMA_ALLOCATOR_HANDLE allocator, VMA_MEMORY_HANDLE allocation) {
    vmaFreeMemory(reinterpret_cast<VmaAllocator>(allocator), reinterpret_cast<VmaAllocation>(allocation));
}

extern "C" VMA_EXPORT void destroy_allocator(VMA_ALLOCATOR_HANDLE allocator)
{
    vmaDestroyAllocator(reinterpret_cast<VmaAllocator>(allocator));
}
