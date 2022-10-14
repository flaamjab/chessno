vertex_shaders=shaders/*.vert
fragment_shaders=shaders/*.frag
shaders="$vertex_shaders $fragment_shaders"

for shader in $shaders; do
  glslc $shader -o $shader.spv
done
