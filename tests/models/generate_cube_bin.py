import struct
import numpy as np

# Vertices for a cube (6 faces, 4 vertices per face)
vertices = np.array([
    # Front face
    [-1, -1,  1], [1, -1,  1], [1,  1,  1], [-1,  1,  1],
    # Back face
    [-1, -1, -1], [-1,  1, -1], [1,  1, -1], [1, -1, -1],
    # Top face
    [-1,  1, -1], [-1,  1,  1], [1,  1,  1], [1,  1, -1],
    # Bottom face
    [-1, -1, -1], [1, -1, -1], [1, -1,  1], [-1, -1,  1],
    # Right face
    [1, -1, -1], [1,  1, -1], [1,  1,  1], [1, -1,  1],
    # Left face
    [-1, -1, -1], [-1, -1,  1], [-1,  1,  1], [-1,  1, -1],
], dtype=np.float32)

# Normals for each face
normals = np.array([
    # Front face
    [0, 0, 1], [0, 0, 1], [0, 0, 1], [0, 0, 1],
    # Back face
    [0, 0, -1], [0, 0, -1], [0, 0, -1], [0, 0, -1],
    # Top face
    [0, 1, 0], [0, 1, 0], [0, 1, 0], [0, 1, 0],
    # Bottom face
    [0, -1, 0], [0, -1, 0], [0, -1, 0], [0, -1, 0],
    # Right face
    [1, 0, 0], [1, 0, 0], [1, 0, 0], [1, 0, 0],
    # Left face
    [-1, 0, 0], [-1, 0, 0], [-1, 0, 0], [-1, 0, 0],
], dtype=np.float32)

# UV coordinates for each vertex
uvs = np.array([
    # Front face
    [0, 0], [1, 0], [1, 1], [0, 1],
    # Back face
    [1, 0], [1, 1], [0, 1], [0, 0],
    # Top face
    [0, 1], [0, 0], [1, 0], [1, 1],
    # Bottom face
    [1, 1], [0, 1], [0, 0], [1, 0],
    # Right face
    [1, 0], [1, 1], [0, 1], [0, 0],
    # Left face
    [0, 0], [1, 0], [1, 1], [0, 1],
], dtype=np.float32)

# Indices for the triangles
indices = np.array([
    0,  1,  2,    0,  2,  3,  # front
    4,  5,  6,    4,  6,  7,  # back
    8,  9,  10,   8,  10, 11, # top
    12, 13, 14,   12, 14, 15, # bottom
    16, 17, 18,   16, 18, 19, # right
    20, 21, 22,   20, 22, 23  # left
], dtype=np.uint16)

# Write the binary file
with open('cube.bin', 'wb') as f:
    # Write vertices
    f.write(vertices.tobytes())
    # Write normals
    f.write(normals.tobytes())
    # Write UVs
    f.write(uvs.tobytes())
    # Write indices
    f.write(indices.tobytes()) 