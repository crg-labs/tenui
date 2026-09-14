struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct InstanceInput {
    @location(1) grid_pos: vec2<f32>,
    @location(2) fg_color: vec4<f32>,
    @location(3) bg_color: vec4<f32>,
    @location(4) glyph_uv_min: vec2<f32>,
    @location(5) glyph_uv_max: vec2<f32>,
    @location(6) attributes: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fg_color: vec4<f32>,
    @location(2) bg_color: vec4<f32>,
    @location(3) @interpolate(flat) attributes: u32,
};

@group(0) @binding(0) var glyph_atlas: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    // Map cell grid coordinates to normalized device coordinates (NDC)
    let cell_size_ndc = vec2<f32>(2.0 / 100.0, 2.0 / 32.0);
    let top_left = vec2<f32>(-1.0, 1.0);
    
    let ndc_pos = top_left + vec2<f32>(
        (instance.grid_pos.x + model.position.x) * cell_size_ndc.x,
        -(instance.grid_pos.y + model.position.y) * cell_size_ndc.y
    );
    
    out.clip_position = vec4<f32>(ndc_pos, 0.0, 1.0);
    out.uv = mix(instance.glyph_uv_min, instance.glyph_uv_max, model.position);
    out.fg_color = instance.fg_color;
    out.bg_color = instance.bg_color;
    out.attributes = instance.attributes;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(glyph_atlas, atlas_sampler, in.uv).r;
    var final_color = mix(in.bg_color, in.fg_color, alpha);
    
    // Hardware CRT Scanline Emulation
    let pixel_y = in.clip_position.y;
    if (u32(pixel_y) % 2u == 1u) {
        final_color = vec4<f32>(final_color.rgb * 0.92, final_color.a);
    }
    
    return final_color;
}
