# WGPU 3D Engine

A modern 3D engine built with Rust and WGPU, featuring physically-based rendering, model loading, and smooth camera controls.

## Features

### Rendering
- Modern WGPU-based renderer with efficient GPU utilization
- Support for GLTF and OBJ model loading
- PBR material system with:
  - Diffuse textures
  - Normal mapping
  - Specular highlights
- Dynamic lighting system:
  - Directional light with adjustable color and direction
  - Ambient light with adjustable intensity
  - Simple ambient occlusion

### Camera System
- Smooth FPS-style camera controls
- Mouse look with pitch and yaw
- WASD movement relative to camera direction
- Space/Shift for vertical movement
- Configurable movement and mouse sensitivity
- Proper view frustum with adjustable FOV

### Model Loading
- Support for GLTF/GLB files with:
  - Multiple meshes per model
  - Material properties
  - Normal maps
  - Texture coordinates
- OBJ file support with:
  - Basic material properties
  - Texture coordinates
  - Normal vectors
  - Auto-generated tangent vectors

### Technical Features
- Modern Rust architecture with safe abstractions
- Efficient vertex and index buffer management
- Proper resource cleanup and GPU memory management
- Comprehensive test suite for core components
- Type-safe shader interface using WGSL

## Getting Started

### Prerequisites
- Rust (latest stable version)
- A GPU with Vulkan, Metal, DX12, or WebGPU support

### Building
```bash
# Clone the repository
git clone [repository-url]
cd 3d-engine

# Build the project
cargo build

# Run the project
cargo run
```

### Controls
- **Mouse**: Look around (hold left click)
- **W/A/S/D**: Move forward/left/backward/right
- **Space**: Move up
- **Left Shift**: Move down
- **Escape**: Release mouse capture

## Architecture

### Core Components

#### Scene Management
- `Scene`: Manages the 3D world state including:
  - Objects and their transforms
  - Camera position and orientation
  - Lighting configuration

#### Rendering Pipeline
- `Renderer`: Handles all GPU interaction:
  - Pipeline state management
  - Shader binding
  - Draw call submission
  - Resource management

#### Asset Management
- `Model`: Handles 3D model loading and management
- `Material`: Manages PBR material properties and textures
- `Texture`: Handles texture loading and GPU upload

#### Camera System
- `Camera`: Manages view and projection:
  - Position and orientation tracking
  - Input processing
  - View matrix calculation
  - Projection configuration

### Shader System
The engine uses WGSL shaders with:
- Vertex processing for mesh transformation
- Fragment processing for PBR lighting
- Normal map calculations
- View/projection transformations

## Contributing
Contributions are welcome! Please feel free to submit pull requests.

## License
[Your chosen license]

## Acknowledgments
- WGPU team for the excellent graphics API
- Rust community for the robust ecosystem
- [Any other acknowledgments] 