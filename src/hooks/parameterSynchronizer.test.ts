import { describe, expect, it, vi } from 'vitest';
import { defaultAudioParameters, type AudioParameters } from '../types/parameters';
import { ParameterSynchronizer, type ParameterSyncState } from './parameterSynchronizer';

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (cause?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

function parameters(changes: Partial<AudioParameters> = {}): AudioParameters {
  return { ...defaultAudioParameters, ...changes };
}

function createSynchronizer(
  getParameters: () => Promise<AudioParameters>,
  setParameters: (next: AudioParameters) => Promise<void>,
) {
  const states: ParameterSyncState[] = [];
  const synchronizer = new ParameterSynchronizer(defaultAudioParameters, {
    getParameters,
    setParameters,
    onStateChange: (state) => states.push(state),
  });
  synchronizer.connect();
  return { synchronizer, states };
}

describe('ParameterSynchronizer', () => {
  it('records a successful parameter update as backend-confirmed', async () => {
    const getParameters = vi.fn().mockResolvedValue(parameters());
    const setParameters = vi.fn().mockResolvedValue(undefined);
    const { synchronizer } = createSynchronizer(getParameters, setParameters);
    await synchronizer.settle();

    synchronizer.update({ pitchSemitones: 3 });
    await synchronizer.settle();

    expect(setParameters).toHaveBeenCalledOnce();
    expect(synchronizer.snapshot()).toEqual({
      parameters: parameters({ pitchSemitones: 3 }),
      confirmedParameters: parameters({ pitchSemitones: 3 }),
      error: null,
    });
  });

  it('restores the last confirmed snapshot when update and reconciliation both fail', async () => {
    const getParameters = vi
      .fn()
      .mockResolvedValueOnce(parameters({ outputGainDb: -8 }))
      .mockRejectedValueOnce(new Error('read failed'));
    const setParameters = vi.fn().mockRejectedValue(new Error('write failed'));
    const { synchronizer } = createSynchronizer(getParameters, setParameters);
    await synchronizer.settle();

    synchronizer.update({ outputGainDb: 6 });
    await synchronizer.settle();

    expect(synchronizer.snapshot().parameters.outputGainDb).toBe(-8);
    expect(synchronizer.snapshot().confirmedParameters.outputGainDb).toBe(-8);
    expect(synchronizer.snapshot().error).toContain('write failed');
    expect(synchronizer.snapshot().error).toContain('read failed');
  });

  it('uses an authoritative snapshot after failure and clears the error on a later success', async () => {
    const authoritative = parameters({ pitchSemitones: -2 });
    const getParameters = vi
      .fn()
      .mockResolvedValueOnce(parameters())
      .mockResolvedValueOnce(authoritative);
    const setParameters = vi
      .fn()
      .mockRejectedValueOnce(new Error('rejected'))
      .mockResolvedValueOnce(undefined);
    const { synchronizer } = createSynchronizer(getParameters, setParameters);
    await synchronizer.settle();

    synchronizer.update({ pitchSemitones: 12 });
    await synchronizer.settle();
    expect(synchronizer.snapshot().parameters).toEqual(authoritative);
    expect(synchronizer.snapshot().error).toContain('Backend settings were restored');

    synchronizer.update({ pitchSemitones: -1 });
    await synchronizer.settle();
    expect(synchronizer.snapshot().error).toBeNull();
    expect(synchronizer.snapshot().confirmedParameters.pitchSemitones).toBe(-1);
  });

  it('coalesces rapid updates behind the request already in flight', async () => {
    const firstRequest = deferred<void>();
    const getParameters = vi.fn().mockResolvedValue(parameters());
    const setParameters = vi
      .fn()
      .mockImplementationOnce(() => firstRequest.promise)
      .mockResolvedValueOnce(undefined);
    const { synchronizer } = createSynchronizer(getParameters, setParameters);
    await synchronizer.settle();

    synchronizer.update({ pitchSemitones: 1 });
    synchronizer.update({ pitchSemitones: 2 });
    synchronizer.update({ pitchSemitones: 3 });
    expect(setParameters).toHaveBeenCalledTimes(1);

    firstRequest.resolve();
    await synchronizer.settle();
    expect(setParameters).toHaveBeenCalledTimes(2);
    expect(setParameters.mock.calls[1][0].pitchSemitones).toBe(3);
    expect(synchronizer.snapshot().parameters.pitchSemitones).toBe(3);
  });

  it('settles a failed update before a preset operation and blocks new slider updates', async () => {
    const getParameters = vi.fn().mockResolvedValue(parameters());
    const setParameters = vi.fn().mockRejectedValue(new Error('rejected'));
    const { synchronizer } = createSynchronizer(getParameters, setParameters);
    await synchronizer.settle();

    synchronizer.update({ pitchSemitones: 8 });
    const presetReady = synchronizer.beginPresetOperation();
    expect(synchronizer.update({ pitchSemitones: 9 })).toBe(false);

    const synchronizedParameters = await presetReady;
    expect(synchronizedParameters.pitchSemitones).toBe(0);
    expect(setParameters).toHaveBeenCalledOnce();

    const presetParameters = parameters({ pitchSemitones: -4 });
    synchronizer.finishPresetOperation(presetParameters);
    expect(synchronizer.snapshot().parameters).toEqual(presetParameters);
    expect(synchronizer.snapshot().error).toBeNull();
  });

  it('ignores an initial load that completes after a newer successful update', async () => {
    const staleLoad = deferred<AudioParameters>();
    const setParameters = vi.fn().mockResolvedValue(undefined);
    const { synchronizer } = createSynchronizer(() => staleLoad.promise, setParameters);

    synchronizer.update({ warmthDb: 4 });
    await Promise.resolve();
    expect(synchronizer.snapshot().confirmedParameters.warmthDb).toBe(4);

    staleLoad.resolve(parameters({ warmthDb: -4 }));
    await synchronizer.settle();
    expect(synchronizer.snapshot().parameters.warmthDb).toBe(4);
    expect(synchronizer.snapshot().confirmedParameters.warmthDb).toBe(4);
  });

  it('does not emit after disconnect or send an update that was still queued', async () => {
    const firstRequest = deferred<void>();
    const getParameters = vi.fn().mockResolvedValue(parameters());
    const setParameters = vi.fn().mockImplementation(() => firstRequest.promise);
    const { synchronizer, states } = createSynchronizer(getParameters, setParameters);
    await synchronizer.settle();

    synchronizer.update({ brightnessDb: 1 });
    synchronizer.update({ brightnessDb: 2 });
    const stateCountAtDisconnect = states.length;
    synchronizer.disconnect();
    firstRequest.resolve();
    await synchronizer.settle();
    await Promise.resolve();

    expect(setParameters).toHaveBeenCalledOnce();
    expect(states).toHaveLength(stateCountAtDisconnect);
    expect(synchronizer.update({ brightnessDb: 3 })).toBe(false);
  });
});
