import type { AudioParameters } from '../types/parameters';

export type ParameterSyncState = {
  parameters: AudioParameters;
  confirmedParameters: AudioParameters;
  error: string | null;
};

type ParameterSynchronizerDependencies = {
  getParameters: () => Promise<AudioParameters>;
  setParameters: (parameters: AudioParameters) => Promise<void>;
  onStateChange: (state: ParameterSyncState) => void;
};

type PendingUpdate = {
  revision: number;
  parameters: AudioParameters;
};

function copyParameters(parameters: AudioParameters): AudioParameters {
  return { ...parameters };
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

export class ParameterSynchronizer {
  private parameters: AudioParameters;
  private confirmedParameters: AudioParameters;
  private error: string | null = null;
  private pending: PendingUpdate | null = null;
  private revision = 0;
  private session = 0;
  private connected = false;
  private updatesBlocked = true;
  private drainRunning = false;
  private requestInFlight = false;
  private reconciling = false;
  private loadingSession: number | null = null;
  private readonly idleWaiters: Array<() => void> = [];

  constructor(
    initialParameters: AudioParameters,
    private readonly dependencies: ParameterSynchronizerDependencies,
  ) {
    this.parameters = copyParameters(initialParameters);
    this.confirmedParameters = copyParameters(initialParameters);
  }

  connect(): void {
    if (this.connected) {
      return;
    }

    this.connected = true;
    this.updatesBlocked = false;
    const session = ++this.session;
    const revision = this.revision;
    this.loadingSession = session;
    this.emit();
    void this.loadInitialParameters(session, revision);
  }

  disconnect(): void {
    if (!this.connected) {
      return;
    }

    this.connected = false;
    this.updatesBlocked = true;
    this.pending = null;
    this.loadingSession = null;
    this.reconciling = false;
    this.revision += 1;
    this.session += 1;
    this.resolveIdleWaiters();
  }

  update(changes: Partial<AudioParameters>): boolean {
    if (!this.connected || this.updatesBlocked) {
      return false;
    }

    this.revision += 1;
    this.parameters = { ...this.parameters, ...changes };
    this.pending = {
      revision: this.revision,
      parameters: copyParameters(this.parameters),
    };
    this.emit();
    this.startDrain();
    return true;
  }

  async settle(): Promise<void> {
    if (!this.connected || this.isIdle()) {
      return;
    }

    await new Promise<void>((resolve) => {
      this.idleWaiters.push(resolve);
    });
  }

  async beginPresetOperation(): Promise<AudioParameters> {
    if (!this.connected) {
      return copyParameters(this.parameters);
    }

    this.updatesBlocked = true;
    await this.settle();
    return copyParameters(this.parameters);
  }

  finishPresetOperation(parameters?: AudioParameters): void {
    if (!this.connected) {
      return;
    }

    if (parameters) {
      this.revision += 1;
      this.pending = null;
      this.parameters = copyParameters(parameters);
      this.confirmedParameters = copyParameters(parameters);
      this.error = null;
    }
    this.updatesBlocked = false;
    this.emit();
    this.resolveIdleWaitersIfIdle();
  }

  snapshot(): ParameterSyncState {
    return {
      parameters: copyParameters(this.parameters),
      confirmedParameters: copyParameters(this.confirmedParameters),
      error: this.error,
    };
  }

  private async loadInitialParameters(session: number, revision: number): Promise<void> {
    try {
      const parameters = await this.dependencies.getParameters();
      if (this.isCurrent(session) && this.revision === revision) {
        this.parameters = copyParameters(parameters);
        this.confirmedParameters = copyParameters(parameters);
        this.error = null;
        this.emit();
      }
    } catch (cause) {
      if (this.isCurrent(session) && this.revision === revision) {
        this.error = `Unable to load audio settings: ${errorMessage(cause)}`;
        this.emit();
      }
    } finally {
      if (this.loadingSession === session) {
        this.loadingSession = null;
        this.resolveIdleWaitersIfIdle();
      }
    }
  }

  private startDrain(): void {
    if (this.drainRunning || !this.connected) {
      return;
    }

    this.drainRunning = true;
    void this.drain();
  }

  private async drain(): Promise<void> {
    try {
      while (this.connected && this.pending) {
        const request = this.pending;
        const session = this.session;
        this.pending = null;
        this.requestInFlight = true;

        try {
          await this.dependencies.setParameters(copyParameters(request.parameters));
          if (this.isCurrent(session)) {
            this.confirmedParameters = copyParameters(request.parameters);
            if (request.revision === this.revision) {
              this.error = null;
            }
            this.emit();
          }
        } catch (cause) {
          if (this.isCurrent(session)) {
            await this.reconcileAfterFailure(request, cause, session);
          }
        } finally {
          this.requestInFlight = false;
        }
      }
    } finally {
      this.drainRunning = false;
      this.resolveIdleWaitersIfIdle();
      if (this.connected && this.pending) {
        this.startDrain();
      }
    }
  }

  private async reconcileAfterFailure(
    request: PendingUpdate,
    updateCause: unknown,
    session: number,
  ): Promise<void> {
    const updateError = errorMessage(updateCause);
    this.error = `Unable to apply audio settings: ${updateError}`;
    this.reconciling = true;
    this.emit();

    try {
      const authoritative = await this.dependencies.getParameters();
      if (!this.isCurrent(session)) {
        return;
      }

      this.confirmedParameters = copyParameters(authoritative);
      if (this.revision === request.revision) {
        this.parameters = copyParameters(authoritative);
      }
      this.error = `Unable to apply audio settings: ${updateError}. Backend settings were restored.`;
      this.emit();
    } catch (reconciliationCause) {
      if (!this.isCurrent(session)) {
        return;
      }

      if (this.revision === request.revision) {
        this.parameters = copyParameters(this.confirmedParameters);
      }
      this.error =
        `Unable to apply audio settings: ${updateError}. ` +
        `Unable to reload backend settings: ${errorMessage(reconciliationCause)}. ` +
        'Restored the last confirmed settings.';
      this.emit();
    } finally {
      if (this.isCurrent(session)) {
        this.reconciling = false;
      }
    }
  }

  private isCurrent(session: number): boolean {
    return this.connected && this.session === session;
  }

  private isIdle(): boolean {
    return (
      !this.drainRunning &&
      !this.requestInFlight &&
      !this.reconciling &&
      this.pending === null &&
      this.loadingSession === null
    );
  }

  private emit(): void {
    if (this.connected) {
      this.dependencies.onStateChange(this.snapshot());
    }
  }

  private resolveIdleWaitersIfIdle(): void {
    if (!this.connected || this.isIdle()) {
      this.resolveIdleWaiters();
    }
  }

  private resolveIdleWaiters(): void {
    const waiters = this.idleWaiters.splice(0);
    waiters.forEach((resolve) => resolve());
  }
}
