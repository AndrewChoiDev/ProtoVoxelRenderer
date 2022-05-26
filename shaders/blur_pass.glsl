#version 450

layout(location = 0) in vec2 tex_coords;
layout(location = 0) out vec3 f_color;
// layout(set = 0, binding = 0, rgba32f) uniform readonly image2D inputColor;
layout (binding = 0) uniform sampler2D samplerColor;
layout (binding = 1) uniform sampler2D inputDepth;

layout (constant_id = 0) const bool vertical = false;

void main()
{
	vec2 dims = vec2(textureSize(samplerColor, 0));
	vec2 inUV = floor(gl_FragCoord.xy) / dims;

    ivec2 coords = ivec2(gl_FragCoord.xy);
    float depth = texture(inputDepth, inUV).r;

    float weight[5];
	weight[0] = 0.227027;
	weight[1] = 0.1945946;
	weight[2] = 0.1216216;
	weight[3] = 0.054054;
	weight[4] = 0.016216;

    float blurStrength = 1.5;
	float blurScale = depth * 0.1;
	blurScale = 0.0;

	vec2 tex_offset = 1.0 / textureSize(samplerColor, 0) * blurScale;
	vec3 result = texture(samplerColor, inUV).rgb * weight[0];
	for(int i = 1; i < 5; ++i)
	{
		if (vertical)
		{
			// V
			result += texture(samplerColor, inUV + vec2(0.0, tex_offset.y * i)).rgb * weight[i] * blurStrength;
			result += texture(samplerColor, inUV - vec2(0.0, tex_offset.y * i)).rgb * weight[i] * blurStrength;
		}
		else
		{
			// H
			result += texture(samplerColor, inUV + vec2(tex_offset.x * i, 0.0)).rgb * weight[i] * blurStrength;
			result += texture(samplerColor, inUV - vec2(tex_offset.x * i, 0.0)).rgb * weight[i] * blurStrength;
		}
    }

    f_color = result;
}