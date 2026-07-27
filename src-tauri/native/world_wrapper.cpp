#include "world/cheaptrick.h"
#include "world/d4c.h"
#include "world/harvest.h"
#include "world/synthesis.h"

#include <algorithm>
#include <atomic>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <limits>
#include <memory>
#include <new>
#include <vector>

namespace {

enum MamWorldStatus {
  MAM_WORLD_OK = 0,
  MAM_WORLD_INVALID_ARGUMENT = 1,
  MAM_WORLD_OVERFLOW = 2,
  MAM_WORLD_ALLOCATION_FAILED = 3,
  MAM_WORLD_EXCEPTION = 4,
  MAM_WORLD_MALFORMED_RESULT = 5,
  MAM_WORLD_BUFFER_TOO_SMALL = 6,
};

struct MamWorldConfiguration {
  double frame_period_ms;
  double f0_floor_hz;
  double f0_ceiling_hz;
};

struct MamWorldMetadata {
  std::uint32_t sample_rate;
  std::size_t frame_count;
  std::size_t fft_size;
  std::size_t bins_per_frame;
  std::size_t raw_synthesis_frames;
  double frame_period_ms;
  double f0_floor_hz;
  double f0_ceiling_hz;
};

struct MamWorldTransformStats {
  std::size_t voiced_frame_count;
  std::size_t clamped_f0_frame_count;
};

struct MamWorldResult {
  MamWorldMetadata metadata{};
  std::vector<double> time_axis;
  std::vector<double> f0;
  std::vector<double> spectral_envelope;
  std::vector<double> aperiodicity;
  bool transformed = false;
};

std::atomic<std::size_t> g_live_results{0};

void SetError(char *buffer, std::size_t capacity, const char *message) noexcept {
  if (buffer == nullptr || capacity == 0) return;
  const std::size_t length = std::min(capacity - 1, std::strlen(message));
  std::memcpy(buffer, message, length);
  buffer[length] = '\0';
}

void ClearError(char *buffer, std::size_t capacity) noexcept {
  if (buffer != nullptr && capacity != 0) buffer[0] = '\0';
}

bool CheckedMultiply(
    std::size_t left,
    std::size_t right,
    std::size_t *result) noexcept {
  if (result == nullptr) return false;
  if (left != 0 && right > std::numeric_limits<std::size_t>::max() / left) {
    return false;
  }
  *result = left * right;
  return true;
}

bool IsConfigurationValid(
    const MamWorldConfiguration &configuration,
    std::uint32_t sample_rate) noexcept {
  const double nyquist = static_cast<double>(sample_rate) * 0.5;
  return std::isfinite(configuration.frame_period_ms) &&
      configuration.frame_period_ms > 0.0 &&
      configuration.frame_period_ms <= 20.0 &&
      std::isfinite(configuration.f0_floor_hz) &&
      std::isfinite(configuration.f0_ceiling_hz) &&
      configuration.f0_floor_hz >= 20.0 &&
      configuration.f0_floor_hz < configuration.f0_ceiling_hz &&
      configuration.f0_ceiling_hz < nyquist;
}

bool ValidateResult(const MamWorldResult &result) noexcept {
  std::size_t matrix_length = 0;
  if (result.metadata.frame_count == 0 ||
      result.metadata.fft_size < 2 ||
      result.metadata.bins_per_frame != result.metadata.fft_size / 2 + 1 ||
      !CheckedMultiply(
          result.metadata.frame_count,
          result.metadata.bins_per_frame,
          &matrix_length) ||
      result.time_axis.size() != result.metadata.frame_count ||
      result.f0.size() != result.metadata.frame_count ||
      result.spectral_envelope.size() != matrix_length ||
      result.aperiodicity.size() != matrix_length) {
    return false;
  }
  for (std::size_t index = 0; index < result.metadata.frame_count; ++index) {
    if (!std::isfinite(result.time_axis[index]) ||
        result.time_axis[index] < 0.0 ||
        !std::isfinite(result.f0[index]) ||
        result.f0[index] < 0.0) {
      return false;
    }
  }
  for (std::size_t index = 0; index < matrix_length; ++index) {
    if (!std::isfinite(result.spectral_envelope[index]) ||
        result.spectral_envelope[index] <= 0.0 ||
        !std::isfinite(result.aperiodicity[index]) ||
        result.aperiodicity[index] < 0.0 ||
        result.aperiodicity[index] > 1.0) {
      return false;
    }
  }
  return true;
}

std::size_t NaturalSynthesisLength(
    std::size_t frame_count,
    double frame_period_ms,
    std::uint32_t sample_rate) noexcept {
  if (frame_count == 0) return 0;
  const long double frames =
      (static_cast<long double>(frame_count - 1) *
       static_cast<long double>(frame_period_ms) *
       static_cast<long double>(sample_rate) / 1000.0L) + 1.0L;
  if (!std::isfinite(static_cast<double>(frames)) ||
      frames <= 0.0L ||
      frames > static_cast<long double>(std::numeric_limits<int>::max())) {
    return 0;
  }
  return static_cast<std::size_t>(frames);
}

bool WarpSpectralEnvelopeRow(
    const double *source,
    std::size_t bins,
    double ratio,
    double *destination) noexcept {
  if (source == nullptr || destination == nullptr || bins < 2 ||
      !std::isfinite(ratio) || ratio <= 0.0) {
    return false;
  }
  for (std::size_t output_bin = 0; output_bin < bins; ++output_bin) {
    const double source_bin =
        std::min(
            static_cast<double>(bins - 1),
            static_cast<double>(output_bin) / ratio);
    const std::size_t lower =
        static_cast<std::size_t>(std::floor(source_bin));
    const std::size_t upper = std::min(lower + 1, bins - 1);
    const double fraction = source_bin - static_cast<double>(lower);
    const double lower_log = std::log(std::max(
        source[lower], std::numeric_limits<double>::min()));
    const double upper_log = std::log(std::max(
        source[upper], std::numeric_limits<double>::min()));
    const double transformed =
        std::exp(lower_log + (upper_log - lower_log) * fraction);
    if (!std::isfinite(transformed) || transformed <= 0.0) {
      return false;
    }
    destination[output_bin] = transformed;
  }
  return true;
}

}  // namespace

extern "C" {

int mam_world_checked_matrix_length(
    std::size_t rows,
    std::size_t columns,
    std::size_t *length) noexcept {
  if (length != nullptr) *length = 0;
  if (rows == 0 || columns == 0 || length == nullptr) {
    return MAM_WORLD_INVALID_ARGUMENT;
  }
  return CheckedMultiply(rows, columns, length)
      ? MAM_WORLD_OK
      : MAM_WORLD_OVERFLOW;
}

int mam_world_warp_spectral_envelope(
    const double *source,
    std::size_t bins,
    double formant_semitones,
    double *destination,
    char *error,
    std::size_t error_capacity) noexcept {
  ClearError(error, error_capacity);
  if (source == nullptr || destination == nullptr || bins < 2 ||
      !std::isfinite(formant_semitones) ||
      formant_semitones < -6.0 || formant_semitones > 6.0) {
    SetError(error, error_capacity, "WORLD envelope warp arguments are invalid.");
    return MAM_WORLD_INVALID_ARGUMENT;
  }
  try {
    const double ratio = std::pow(2.0, formant_semitones / 12.0);
    const std::vector<double> immutable_source(source, source + bins);
    if (!WarpSpectralEnvelopeRow(
            immutable_source.data(), bins, ratio, destination)) {
      SetError(error, error_capacity, "WORLD envelope warp produced invalid values.");
      return MAM_WORLD_MALFORMED_RESULT;
    }
    return MAM_WORLD_OK;
  } catch (const std::bad_alloc &) {
    SetError(error, error_capacity, "WORLD envelope warp allocation failed.");
    return MAM_WORLD_ALLOCATION_FAILED;
  } catch (...) {
    SetError(error, error_capacity, "WORLD envelope warp failed with a contained native exception.");
    return MAM_WORLD_EXCEPTION;
  }
}

int mam_world_analyze(
    const float *samples,
    std::size_t sample_count,
    std::uint32_t sample_rate,
    const MamWorldConfiguration *configuration,
    MamWorldResult **output,
    char *error,
    std::size_t error_capacity) noexcept {
  ClearError(error, error_capacity);
  if (output != nullptr) *output = nullptr;
  if (samples == nullptr || sample_count == 0 || sample_rate == 0 ||
      configuration == nullptr || output == nullptr) {
    SetError(error, error_capacity, "WORLD analysis received an invalid pointer, length, or sample rate.");
    return MAM_WORLD_INVALID_ARGUMENT;
  }
  if (sample_count > static_cast<std::size_t>(std::numeric_limits<int>::max()) ||
      sample_rate > static_cast<std::uint32_t>(std::numeric_limits<int>::max()) ||
      !IsConfigurationValid(*configuration, sample_rate)) {
    SetError(error, error_capacity, "WORLD analysis configuration is unsupported or exceeds native limits.");
    return MAM_WORLD_INVALID_ARGUMENT;
  }

  try {
    std::unique_ptr<MamWorldResult> result(new (std::nothrow) MamWorldResult);
    if (!result) {
      SetError(error, error_capacity, "WORLD could not allocate its result handle.");
      return MAM_WORLD_ALLOCATION_FAILED;
    }

    std::vector<double> input;
    input.reserve(sample_count);
    for (std::size_t index = 0; index < sample_count; ++index) {
      if (!std::isfinite(samples[index])) {
        SetError(error, error_capacity, "WORLD input contains a non-finite sample.");
        return MAM_WORLD_INVALID_ARGUMENT;
      }
      input.push_back(static_cast<double>(samples[index]));
    }

    const int native_sample_rate = static_cast<int>(sample_rate);
    const int native_sample_count = static_cast<int>(sample_count);
    const int native_frame_count = GetSamplesForHarvest(
        native_sample_rate,
        native_sample_count,
        configuration->frame_period_ms);
    if (native_frame_count <= 0) {
      SetError(error, error_capacity, "WORLD Harvest returned an invalid frame count.");
      return MAM_WORLD_MALFORMED_RESULT;
    }

    HarvestOption harvest_option{};
    InitializeHarvestOption(&harvest_option);
    harvest_option.frame_period = configuration->frame_period_ms;
    harvest_option.f0_floor = configuration->f0_floor_hz;
    harvest_option.f0_ceil = configuration->f0_ceiling_hz;

    CheapTrickOption cheaptrick_option{};
    InitializeCheapTrickOption(native_sample_rate, &cheaptrick_option);
    cheaptrick_option.f0_floor = configuration->f0_floor_hz;
    cheaptrick_option.fft_size =
        GetFFTSizeForCheapTrick(native_sample_rate, &cheaptrick_option);
    if (cheaptrick_option.fft_size <= 0) {
      SetError(error, error_capacity, "WORLD CheapTrick returned an invalid FFT size.");
      return MAM_WORLD_MALFORMED_RESULT;
    }

    const std::size_t frame_count = static_cast<std::size_t>(native_frame_count);
    const std::size_t fft_size =
        static_cast<std::size_t>(cheaptrick_option.fft_size);
    const std::size_t bins_per_frame = fft_size / 2 + 1;
    std::size_t matrix_length = 0;
    if (!CheckedMultiply(frame_count, bins_per_frame, &matrix_length) ||
        matrix_length > std::vector<double>().max_size()) {
      SetError(error, error_capacity, "WORLD matrix dimensions overflow addressable memory.");
      return MAM_WORLD_OVERFLOW;
    }

    result->metadata.sample_rate = sample_rate;
    result->metadata.frame_count = frame_count;
    result->metadata.fft_size = fft_size;
    result->metadata.bins_per_frame = bins_per_frame;
    result->metadata.frame_period_ms = configuration->frame_period_ms;
    result->metadata.f0_floor_hz = configuration->f0_floor_hz;
    result->metadata.f0_ceiling_hz = configuration->f0_ceiling_hz;
    result->metadata.raw_synthesis_frames = NaturalSynthesisLength(
        frame_count, configuration->frame_period_ms, sample_rate);
    if (result->metadata.raw_synthesis_frames == 0) {
      SetError(error, error_capacity, "WORLD synthesis length exceeds native limits.");
      return MAM_WORLD_OVERFLOW;
    }

    result->time_axis.resize(frame_count);
    result->f0.resize(frame_count);
    result->spectral_envelope.resize(matrix_length);
    result->aperiodicity.resize(matrix_length);
    std::vector<double *> spectral_rows(frame_count);
    std::vector<double *> aperiodicity_rows(frame_count);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
      spectral_rows[frame] =
          result->spectral_envelope.data() + frame * bins_per_frame;
      aperiodicity_rows[frame] =
          result->aperiodicity.data() + frame * bins_per_frame;
    }

    Harvest(
        input.data(), native_sample_count, native_sample_rate, &harvest_option,
        result->time_axis.data(), result->f0.data());
    CheapTrick(
        input.data(), native_sample_count, native_sample_rate,
        result->time_axis.data(), result->f0.data(), native_frame_count,
        &cheaptrick_option, spectral_rows.data());
    D4COption d4c_option{};
    InitializeD4COption(&d4c_option);
    D4C(
        input.data(), native_sample_count, native_sample_rate,
        result->time_axis.data(), result->f0.data(), native_frame_count,
        cheaptrick_option.fft_size, &d4c_option, aperiodicity_rows.data());

    if (!ValidateResult(*result)) {
      SetError(error, error_capacity, "WORLD analysis produced malformed or non-finite features.");
      return MAM_WORLD_MALFORMED_RESULT;
    }
    g_live_results.fetch_add(1, std::memory_order_relaxed);
    *output = result.release();
    return MAM_WORLD_OK;
  } catch (const std::bad_alloc &) {
    SetError(error, error_capacity, "WORLD analysis allocation failed.");
    return MAM_WORLD_ALLOCATION_FAILED;
  } catch (...) {
    SetError(error, error_capacity, "WORLD analysis failed with a contained native exception.");
    return MAM_WORLD_EXCEPTION;
  }
}

void mam_world_destroy(MamWorldResult *result) noexcept {
  if (result == nullptr) return;
  delete result;
  g_live_results.fetch_sub(1, std::memory_order_relaxed);
}

std::size_t mam_world_live_result_count() noexcept {
  return g_live_results.load(std::memory_order_relaxed);
}

int mam_world_get_metadata(
    const MamWorldResult *result,
    MamWorldMetadata *metadata,
    char *error,
    std::size_t error_capacity) noexcept {
  ClearError(error, error_capacity);
  if (metadata != nullptr) *metadata = MamWorldMetadata{};
  if (result == nullptr || metadata == nullptr) {
    SetError(error, error_capacity, "WORLD metadata received a null pointer.");
    return MAM_WORLD_INVALID_ARGUMENT;
  }
  if (!ValidateResult(*result)) {
    SetError(error, error_capacity, "WORLD result metadata or dimensions are malformed.");
    return MAM_WORLD_MALFORMED_RESULT;
  }
  *metadata = result->metadata;
  return MAM_WORLD_OK;
}

const double *mam_world_time_axis(const MamWorldResult *result) noexcept {
  return result == nullptr ? nullptr : result->time_axis.data();
}

const double *mam_world_f0(const MamWorldResult *result) noexcept {
  return result == nullptr ? nullptr : result->f0.data();
}

const double *mam_world_spectral_envelope(
    const MamWorldResult *result) noexcept {
  return result == nullptr ? nullptr : result->spectral_envelope.data();
}

const double *mam_world_aperiodicity(const MamWorldResult *result) noexcept {
  return result == nullptr ? nullptr : result->aperiodicity.data();
}

int mam_world_transform(
    MamWorldResult *result,
    double pitch_semitones,
    double formant_semitones,
    MamWorldTransformStats *stats,
    char *error,
    std::size_t error_capacity) noexcept {
  ClearError(error, error_capacity);
  if (stats != nullptr) *stats = MamWorldTransformStats{};
  if (result == nullptr || stats == nullptr ||
      !std::isfinite(pitch_semitones) ||
      !std::isfinite(formant_semitones) ||
      pitch_semitones < -12.0 || pitch_semitones > 12.0 ||
      formant_semitones < -6.0 || formant_semitones > 6.0) {
    SetError(error, error_capacity, "WORLD transform parameters are invalid or unsupported.");
    return MAM_WORLD_INVALID_ARGUMENT;
  }
  if (!ValidateResult(*result) || result->transformed) {
    SetError(error, error_capacity, "WORLD result is malformed or was already transformed.");
    return MAM_WORLD_MALFORMED_RESULT;
  }

  try {
    const double pitch_ratio = std::pow(2.0, pitch_semitones / 12.0);
    const double maximum_f0 =
        std::min(
            result->metadata.f0_ceiling_hz * 2.0,
            static_cast<double>(result->metadata.sample_rate) * 0.225);
    for (double &f0 : result->f0) {
      if (f0 == 0.0) continue;
      ++stats->voiced_frame_count;
      double transformed = f0 * pitch_ratio;
      if (transformed < 20.0) {
        transformed = 20.0;
        ++stats->clamped_f0_frame_count;
      } else if (transformed > maximum_f0) {
        transformed = maximum_f0;
        ++stats->clamped_f0_frame_count;
      }
      f0 = transformed;
    }

    if (formant_semitones != 0.0) {
      const double ratio = std::pow(2.0, formant_semitones / 12.0);
      const std::size_t bins = result->metadata.bins_per_frame;
      std::vector<double> source(bins);
      for (std::size_t frame = 0;
           frame < result->metadata.frame_count;
           ++frame) {
        double *destination =
            result->spectral_envelope.data() + frame * bins;
        std::copy(destination, destination + bins, source.begin());
        if (!WarpSpectralEnvelopeRow(
                source.data(), bins, ratio, destination)) {
          SetError(error, error_capacity, "WORLD formant warp produced invalid values.");
          return MAM_WORLD_MALFORMED_RESULT;
        }
      }
    }
    result->transformed = true;
    if (!ValidateResult(*result)) {
      SetError(error, error_capacity, "WORLD transform produced malformed features.");
      return MAM_WORLD_MALFORMED_RESULT;
    }
    return MAM_WORLD_OK;
  } catch (const std::bad_alloc &) {
    SetError(error, error_capacity, "WORLD transform allocation failed.");
    return MAM_WORLD_ALLOCATION_FAILED;
  } catch (...) {
    SetError(error, error_capacity, "WORLD transform failed with a contained native exception.");
    return MAM_WORLD_EXCEPTION;
  }
}

int mam_world_synthesize(
    const MamWorldResult *result,
    float *output,
    std::size_t output_capacity,
    std::size_t *written,
    char *error,
    std::size_t error_capacity) noexcept {
  ClearError(error, error_capacity);
  if (written != nullptr) *written = 0;
  if (result == nullptr || output == nullptr || written == nullptr ||
      output_capacity == 0) {
    SetError(error, error_capacity, "WORLD synthesis received an invalid pointer or zero output length.");
    return MAM_WORLD_INVALID_ARGUMENT;
  }
  if (!ValidateResult(*result) ||
      result->metadata.raw_synthesis_frames == 0 ||
      result->metadata.raw_synthesis_frames >
          static_cast<std::size_t>(std::numeric_limits<int>::max())) {
    SetError(error, error_capacity, "WORLD synthesis result dimensions are malformed.");
    return MAM_WORLD_MALFORMED_RESULT;
  }
  if (output_capacity < result->metadata.raw_synthesis_frames) {
    SetError(error, error_capacity, "WORLD synthesis output buffer is too small.");
    return MAM_WORLD_BUFFER_TOO_SMALL;
  }

  std::fill(output, output + output_capacity, 0.0F);
  try {
    const std::size_t frames = result->metadata.frame_count;
    const std::size_t bins = result->metadata.bins_per_frame;
    std::vector<const double *> spectral_rows(frames);
    std::vector<const double *> aperiodicity_rows(frames);
    for (std::size_t frame = 0; frame < frames; ++frame) {
      spectral_rows[frame] =
          result->spectral_envelope.data() + frame * bins;
      aperiodicity_rows[frame] =
          result->aperiodicity.data() + frame * bins;
    }
    std::vector<double> native_output(result->metadata.raw_synthesis_frames);
    Synthesis(
        result->f0.data(),
        static_cast<int>(frames),
        spectral_rows.data(),
        aperiodicity_rows.data(),
        static_cast<int>(result->metadata.fft_size),
        result->metadata.frame_period_ms,
        static_cast<int>(result->metadata.sample_rate),
        static_cast<int>(result->metadata.raw_synthesis_frames),
        native_output.data());
    for (std::size_t index = 0; index < native_output.size(); ++index) {
      if (!std::isfinite(native_output[index])) {
        SetError(error, error_capacity, "WORLD synthesis produced a non-finite sample.");
        std::fill(output, output + output_capacity, 0.0F);
        return MAM_WORLD_MALFORMED_RESULT;
      }
      output[index] = static_cast<float>(native_output[index]);
      if (!std::isfinite(output[index])) {
        SetError(error, error_capacity, "WORLD synthesis exceeded the finite float range.");
        std::fill(output, output + output_capacity, 0.0F);
        return MAM_WORLD_MALFORMED_RESULT;
      }
    }
    *written = native_output.size();
    return MAM_WORLD_OK;
  } catch (const std::bad_alloc &) {
    SetError(error, error_capacity, "WORLD synthesis allocation failed.");
    return MAM_WORLD_ALLOCATION_FAILED;
  } catch (...) {
    SetError(error, error_capacity, "WORLD synthesis failed with a contained native exception.");
    return MAM_WORLD_EXCEPTION;
  }
}

}  // extern "C"
