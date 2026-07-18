#include <signalsmith-stretch.h>

#include <cstddef>

class InterleavedBuffer {
public:
    InterleavedBuffer(float *data, int channels) : data(data), channels(channels) {}

    class ChannelView {
    public:
        ChannelView(float *data, int channel, int stride)
            : data(data), channel(channel), stride(stride) {}

        float &operator[](std::size_t offset) {
            return data[(offset * stride) + channel];
        }

    private:
        float *data;
        int channel;
        int stride;
    };

    ChannelView operator[](std::size_t channel) {
        return ChannelView(data, static_cast<int>(channel), channels);
    }

private:
    float *data;
    int channels;
};

struct MamSignalsmithStretch {
    signalsmith::stretch::SignalsmithStretch<float> instance;
    int channels;
};

extern "C" {

MamSignalsmithStretch *mam_signalsmith_create(
    int channels,
    std::size_t block_frames,
    std::size_t interval_frames
) {
    auto *handle = new MamSignalsmithStretch;
    handle->channels = channels;
    handle->instance.configure(
        channels,
        static_cast<int>(block_frames),
        static_cast<int>(interval_frames)
    );
    return handle;
}

void mam_signalsmith_destroy(MamSignalsmithStretch *handle) {
    delete handle;
}

void mam_signalsmith_reset(MamSignalsmithStretch *handle) {
    handle->instance.reset();
}

std::size_t mam_signalsmith_input_latency(MamSignalsmithStretch *handle) {
    return static_cast<std::size_t>(handle->instance.inputLatency());
}

std::size_t mam_signalsmith_output_latency(MamSignalsmithStretch *handle) {
    return static_cast<std::size_t>(handle->instance.outputLatency());
}

void mam_signalsmith_set_pitch_semitones(
    MamSignalsmithStretch *handle,
    float semitones
) {
    handle->instance.setTransposeSemitones(semitones);
}

void mam_signalsmith_set_formant_semitones(
    MamSignalsmithStretch *handle,
    float semitones,
    bool compensate_pitch
) {
    handle->instance.setFormantSemitones(semitones, compensate_pitch);
}

void mam_signalsmith_process(
    MamSignalsmithStretch *handle,
    float *input,
    std::size_t input_frames,
    float *output,
    std::size_t output_frames
) {
    InterleavedBuffer input_buffer(input, handle->channels);
    InterleavedBuffer output_buffer(output, handle->channels);
    handle->instance.process(
        input_buffer,
        static_cast<int>(input_frames),
        output_buffer,
        static_cast<int>(output_frames)
    );
}

}

