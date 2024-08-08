// const int kernel_sizes[5] = int[5](11, 11, 20, 26, 64);
// const int kernel_lengths[5] = int[5](121, 121, 400, 26*26, 64*64);


float bell(float x) {
    return exp(-pow((x - m)/s, 2.0)/2.0);
}

float growth(float U) {
    return (bell(U)*2.0) - 1.0;
}

void main() {

    vec2 texCoord = gl_FragCoord.xy / u_resolution;
    vec2 texelSize = 1.0 / u_resolution;

    // this is basically a convolution 2D
    float U = 0.0;

    for (int i = 0; i < kernel_length; i++){
        vec2 offset = vec2(float(i % kernel_size - R), float(i / kernel_size - R)) * texelSize;
        vec2 wrappedCoord = texCoord + offset;
        wrappedCoord = mod(wrappedCoord, 1.0);
        vec4 neighbor = texture(u_current_state, wrappedCoord);
        U += neighbor.r * kernel[i];
    }
    // end of convolution 2D

    vec4 A = texture(u_current_state, texCoord);

    // clamp the new state to 0.0 or 1.0
    float new_state = clamp(A.r + 1.0/T * growth(U), 0.0, 1.0);

    outColor = vec4(new_state, new_state, U, 1.0);

    // if we are at at 0,0 -> kernel_size,kernel_size then draw the kernel
    // if (gl_FragCoord.x < float(kernel_size) && gl_FragCoord.y < float(kernel_size)) {
    //     outColor = vec4(all_kernels[kernel_offset + int(gl_FragCoord.y) * kernel_size + int(gl_FragCoord.x)]*50.0, 0.0, 0.0, 1.0);
    // }

}
