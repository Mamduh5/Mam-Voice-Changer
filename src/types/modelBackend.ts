export type ModelDevice = 'cpu' | 'cuda' | 'directMl';
export type ModelPrecision = 'float32' | 'float16' | 'bfloat16';

export type SeedVcBackendConfiguration = {
  pythonExecutable: string;
  workerPackageDirectory: string;
  seedVcDirectory: string;
  modelConfigurationPath: string;
  pretrainedCheckpointPaths: string[];
  outputDirectory: string;
  device: ModelDevice;
  precision: ModelPrecision;
};

export type ModelBackendSettings = {
  schemaVersion: number;
  seedVc: SeedVcBackendConfiguration | null;
};

export type BackendReadiness =
  | 'notConfigured'
  | 'pythonMissing'
  | 'workerMissing'
  | 'backendMissing'
  | 'checkpointMissing'
  | 'configurationInvalid'
  | 'protocolMismatch'
  | 'unsupportedHardware'
  | 'ready';

export type BackendResourceReport = {
  systemMemoryBytes: number | null;
  gpuMemoryBytes: number | null;
  availableDiskBytes: number | null;
  snapshotSizeBytes: number | null;
  checkpointSizeBytes: number | null;
  riskLevel: string | null;
};

export type BackendCapabilityReport = {
  backendId: string;
  backendVersion: string;
  workerVersion: string;
  protocolVersion: number;
  devices: ModelDevice[];
  precisions: ModelPrecision[];
  supportsResume: boolean;
  supportsMultipleReferences: boolean;
  resources: BackendResourceReport;
  warnings: string[];
};

export type BackendValidationStatus = {
  readiness: BackendReadiness;
  message: string;
  capabilityReport: BackendCapabilityReport | null;
};

export const emptyBackendSettings: ModelBackendSettings = { schemaVersion: 1, seedVc: null };
