#version 300 es
precision highp float;

uniform vec2 u_mouse;
uniform sampler2D u_current_state;
uniform vec2 u_resolution;

out vec4 outColor;

void main() {
    vec2 texCoord = gl_FragCoord.xy / u_resolution;
    vec2 texelSize = vec2(1.0, 1.0) / u_resolution;

    int alive_neighbors = 0;
    for (int x = -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            if (x == 0 && y == 0) continue;
            vec2 offset = vec2(float(x), float(y)) * texelSize;
            vec4 neighbor = texture(u_current_state, texCoord + offset);
            if (neighbor.r > 0.0) alive_neighbors++;
        }
    }

    vec4 current = texture(u_current_state, texCoord);
    bool is_alive = current.r > 0.0;

    if (is_alive) {
        if (alive_neighbors == 2 || alive_neighbors == 3) {
            outColor = vec4(texCoord.x, texCoord.y, 1.0, 1.0);  // Cell stays alive
        } else {
            outColor = vec4(0.0, 0.0, 0.0, 1.0);  // Cell dies
        }
    } else {
        if (alive_neighbors == 3) {
            outColor = vec4(texCoord.x, texCoord.y, 1.0, 1.0);  // Cell becomes alive
        } else {
            outColor = vec4(0.0, 0.0, 0.0, 1.0);  // Cell stays dead
        }
    }

    float dist = distance(gl_FragCoord.xy, u_mouse);
        vec4 color = mix(outColor, vec4(1.0, 0.0, 0.0, 1.0), smoothstep(5.0,0.0, dist));
        outColor = color;
}