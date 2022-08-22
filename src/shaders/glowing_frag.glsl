#version 100

precision lowp float;
varying vec2 uv;

uniform sampler2D Texture;

uniform float time;
uniform float glowFrequency;
uniform float glowIntensity;
uniform float zoomFactor;

#define PI 3.1415926538
#define PI_HALF (PI / 2.0)

float sin01(float x) {
	return (sin(x - PI_HALF) + 1.0) / 2.0;
}

vec2 zoom_uv(vec2 v, float zoom) {
	vec2 full = vec2(1.0, 1.0);
	return ((v * 2.0) - full) * zoom / 2.0 + (full / 2.0);
}

void main() {
	float glowFactor = sin01(time * glowFrequency * PI) * glowIntensity;
    gl_FragColor = texture2D(Texture, zoom_uv(uv, 1.0 / (1.0 + glowFactor * zoomFactor))) * (1.0 + glowFactor);
}
