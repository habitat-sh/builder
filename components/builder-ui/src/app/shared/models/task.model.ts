import { BuildStatus } from './build.model';
import { PackageIdent, PackageTarget } from './package.model';

/**
 * Task status enum
 */
export enum TaskStatus {
  Pending = 'Pending',
  Running = 'Running',
  Complete = 'Complete',
  Failed = 'Failed',
  Canceled = 'Canceled'
}

/**
 * Task priority enum
 */
export enum TaskPriority {
  Low = 'Low',
  Medium = 'Medium',
  High = 'High',
  Critical = 'Critical'
}

/**
 * Task type enum
 */
export enum TaskType {
  Build = 'Build',
  Test = 'Test',
  Deploy = 'Deploy',
  Promote = 'Promote',
  Custom = 'Custom'
}

/**
 * Task model
 */
export interface Task {
  id: string;
  name: string;
  description?: string;
  type: TaskType;
  status: TaskStatus;
  priority: TaskPriority;
  createdAt: Date;
  startedAt?: Date;
  completedAt?: Date;
  createdBy: string;
  assignedTo?: string;
  relatedBuildId?: string;
  relatedPackage?: PackageIdent;
  target?: PackageTarget;
  isRecurring?: boolean;
  recurringSchedule?: string;
  params?: Record<string, any>;
  progress?: number;
  estimatedDuration?: number;
  dependencies?: string[];
}

/**
 * Task output line
 */
export interface TaskOutput {
  line: string;
  timestamp: Date;
  stream: 'stdout' | 'stderr';
}

/**
 * Task log
 */
export interface TaskLog {
  id: string;
  output: TaskOutput[];
  complete: boolean;
}

/**
 * Task search parameters
 */
export interface TaskSearch {
  status?: TaskStatus;
  type?: TaskType;
  priority?: TaskPriority;
  createdBy?: string;
  assignedTo?: string;
  startDate?: Date;
  endDate?: Date;
  keyword?: string;
  limit?: number;
  offset?: number;
}

/**
 * Task search result
 */
export interface TaskSearchResult {
  tasks: Task[];
  total: number;
  offset: number;
  limit: number;
}

/**
 * New task request
 */
export interface NewTaskRequest {
  name: string;
  description?: string;
  type: TaskType;
  priority: TaskPriority;
  assignedTo?: string;
  relatedBuildId?: string;
  relatedPackage?: PackageIdent;
  target?: PackageTarget;
  isRecurring?: boolean;
  recurringSchedule?: string;
  params?: Record<string, any>;
  estimatedDuration?: number;
  dependencies?: string[];
}

/**
 * Task update request
 */
export interface TaskUpdateRequest {
  name?: string;
  description?: string;
  status?: TaskStatus;
  priority?: TaskPriority;
  assignedTo?: string;
  progress?: number;
  params?: Record<string, any>;
}
