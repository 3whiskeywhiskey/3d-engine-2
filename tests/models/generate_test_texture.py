from PIL import Image, ImageDraw

# Create a 256x256 test texture
size = 256
img = Image.new('RGBA', (size, size), color='white')
draw = ImageDraw.Draw(img)

# Draw a grid pattern
grid_size = 32
for i in range(0, size, grid_size):
    draw.line([(i, 0), (i, size)], fill='black', width=1)
    draw.line([(0, i), (size, i)], fill='black', width=1)

# Draw diagonal lines
draw.line([(0, 0), (size, size)], fill='red', width=2)
draw.line([(size, 0), (0, size)], fill='blue', width=2)

# Add some colored squares
square_size = grid_size * 2
colors = ['red', 'green', 'blue', 'yellow']
for i, color in enumerate(colors):
    x = i * square_size
    y = i * square_size
    draw.rectangle([x, y, x + square_size, y + square_size], fill=color)

# Save the texture
img.save('cube_texture.png') 