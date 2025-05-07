import { PackageIdent, PackageTarget } from './package.model';

/**
 * Build status enum
 */
export enum BuildStatus {
  Pending = 'Pending',
  Dispatched = 'Dispatched', 
  InProgress = 'InProgress',
  Complete = 'Complete',
  Failed = 'Failed',
  Canceled = 'Canceled'
}

/**
 * Build job model
 */
export interface BuildJob {
  id: string;
  groupId: string;
  projectId: string;
  originId: string;
  originName: string;
  packageIdent: PackageIdent;
  target: PackageTarget;
  status: BuildStatus;
  createdAt: Date;
  dispatchedAt?: Date;
  startedAt?: Date;
  completedAt?: Date;
  channel?: string;
  isExcluded?: boolean;
}

/**
 * Build output line
 */
export interface BuildOutput {
  line: string;
  timestamp: Date;
  stream: 'stdout' | 'stderr';
}

/**
 * Build log
 */
export interface BuildLog {
  id: string;
  output: BuildOutput[];
  complete: boolean;
}

/**
 * Build search parameters
 */
export interface BuildSearch {
  origin?: string;
  name?: string;
  status?: BuildStatus[];
  page?: number;
  limit?: number;
  fromDate?: Date;
  toDate?: Date;
}

/**
 * Build configuration
 */
export interface BuildConfig {
  id: string;
  installationId: string;
  project: ProjectConfig;
}

/**
 * Project configuration
 */
export interface ProjectConfig {
  path: string;
  plan_path: string;
  auto_build: boolean;
  target?: string;
  use_worker_auth?: boolean;
}

/**
 * Project model
 */
export interface Project {
  id: string;
  name: string;
  originId: string;
  originName: string;
  packageName: string;
  planPath: string;
  vcs: {
    type: string;
    url: string;
    installationId: string;
  };
  visibility: 'public' | 'private';
  ownerId: string;
  createdAt: Date;
  updatedAt: Date;
  autoPublish: boolean;
  autoBuild: boolean;
}

/**
 * Project search parameters
 */
export interface ProjectSearch {
  origin?: string;
  name?: string;
  page?: number;
  limit?: number;
}
