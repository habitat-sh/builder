import { Injectable, Signal, computed, signal } from '@angular/core';
import { EMPTY, catchError, tap } from 'rxjs';
import { NotificationService } from '../../core/services/notification.service';
import { LoadingService } from '../../core/services/loading.service';
import { BuildService } from '../services/build.service';
import {
  BuildJob,
  BuildLog,
  BuildSearch,
  BuildStatus,
  Project,
  ProjectConfig,
  ProjectSearch
} from '../models/build.model';

@Injectable({
  providedIn: 'root'
})
export class BuildState {
  // State signals
  private readonly _currentBuildJob = signal<BuildJob | null>(null);
  private readonly _buildLogs = signal<BuildLog | null>(null);
  private readonly _buildJobs = signal<BuildJob[]>([]);
  private readonly _currentProject = signal<Project | null>(null);
  private readonly _projects = signal<Project[]>([]);
  private readonly _projectConfig = signal<ProjectConfig | null>(null);
  private readonly _error = signal<string | null>(null);
  private readonly _isLoading = signal<boolean>(false);

  // Public readable signals
  public readonly currentBuildJob = this._currentBuildJob.asReadonly();
  public readonly buildLogs = this._buildLogs.asReadonly();
  public readonly buildJobs = this._buildJobs.asReadonly();
  public readonly currentProject = this._currentProject.asReadonly();
  public readonly projects = this._projects.asReadonly();
  public readonly projectConfig = this._projectConfig.asReadonly();
  public readonly error = this._error.asReadonly();
  public readonly isLoading = this._isLoading.asReadonly();

  // Computed signals
  public readonly isComplete: Signal<boolean> = computed(() => 
    !!this._currentBuildJob() && this._currentBuildJob()!.status === BuildStatus.Complete);
  
  public readonly isFailed: Signal<boolean> = computed(() => 
    !!this._currentBuildJob() && this._currentBuildJob()!.status === BuildStatus.Failed);

  public readonly isInProgress: Signal<boolean> = computed(() => 
    !!this._currentBuildJob() && 
    [BuildStatus.Pending, BuildStatus.Dispatched, BuildStatus.InProgress].includes(this._currentBuildJob()!.status));

  constructor(
    private buildService: BuildService,
    private loadingService: LoadingService,
    private notificationService: NotificationService
  ) {}

  /**
   * Loads a build job
   */
  loadBuildJob(id: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.buildService.getBuildJob(id)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load build job');
          this.notificationService.error('Failed to load build job', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(buildJob => {
        this._currentBuildJob.set(buildJob);
        this._isLoading.set(false);
      });
  }

  /**
   * Loads build logs
   */
  loadBuildLogs(id: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.buildService.getBuildLogs(id)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load build logs');
          this.notificationService.error('Failed to load build logs', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(logs => {
        this._buildLogs.set(logs);
        this._isLoading.set(false);
      });
  }

  /**
   * Schedules a build
   */
  scheduleBuild(origin: string, name: string, target?: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.buildService.scheduleBuild(origin, name, target)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to schedule build');
          this.notificationService.error('Failed to schedule build', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(buildJob => {
        this._currentBuildJob.set(buildJob);
        this._isLoading.set(false);
        this.notificationService.success(`Build for ${origin}/${name} scheduled successfully`);
        
        // Refresh build jobs list if we have one
        if (this._buildJobs().length > 0) {
          this.searchBuilds({
            origin,
            name
          });
        }
      });
  }

  /**
   * Cancels a build
   */
  cancelBuild(id: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.buildService.cancelBuild(id)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to cancel build');
          this.notificationService.error('Failed to cancel build', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(() => {
        // Update current build job status if it's the one we're looking at
        if (this._currentBuildJob() && this._currentBuildJob()!.id === id) {
          const currentJob = this._currentBuildJob()!;
          this._currentBuildJob.set({
            ...currentJob,
            status: BuildStatus.Canceled
          });
        }
        
        this._isLoading.set(false);
        this.notificationService.success('Build job was cancelled successfully');
        
        // Refresh build jobs list if we have one
        if (this._buildJobs().length > 0) {
          this.searchBuilds({});
        }
      });
  }

  /**
   * Searches for builds
   */
  searchBuilds(search: BuildSearch): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.buildService.searchBuilds(search)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to search builds');
          this.notificationService.error('Failed to search builds', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(jobs => {
        this._buildJobs.set(jobs);
        this._isLoading.set(false);
      });
  }

  /**
   * Loads projects for an origin
   */
  loadProjects(origin: string, search: ProjectSearch = {}): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.buildService.getProjects(origin, search)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load projects');
          this.notificationService.error('Failed to load projects', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(projects => {
        this._projects.set(projects);
        this._isLoading.set(false);
      });
  }

  /**
   * Loads a project
   */
  loadProject(origin: string, name: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.buildService.getProject(origin, name)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load project');
          this.notificationService.error('Failed to load project', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(project => {
        this._currentProject.set(project);
        this._isLoading.set(false);
      });
  }

  /**
   * Loads project configuration
   */
  loadProjectConfig(origin: string, name: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.buildService.getProjectConfig(origin, name)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load project configuration');
          this.notificationService.error('Failed to load project configuration', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(config => {
        this._projectConfig.set(config);
        this._isLoading.set(false);
      });
  }

  /**
   * Creates a project
   */
  createProject(project: Partial<Project>): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.buildService.createProject(project)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to create project');
          this.notificationService.error('Failed to create project', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(newProject => {
        this._currentProject.set(newProject);
        this._isLoading.set(false);
        this.notificationService.success(`Project ${newProject.name} created successfully`);
        
        // Refresh projects list if it exists
        if (this._projects().length > 0 && project.originName) {
          this.loadProjects(project.originName);
        }
      });
  }

  /**
   * Updates a project
   */
  updateProject(project: Partial<Project>): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.buildService.updateProject(project)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to update project');
          this.notificationService.error('Failed to update project', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(updatedProject => {
        this._currentProject.set(updatedProject);
        this._isLoading.set(false);
        this.notificationService.success(`Project ${updatedProject.name} updated successfully`);
        
        // Refresh projects list if it exists
        if (this._projects().length > 0 && project.originName) {
          this.loadProjects(project.originName);
        }
      });
  }

  /**
   * Resets the state
   */
  reset(): void {
    this._currentBuildJob.set(null);
    this._buildLogs.set(null);
    this._buildJobs.set([]);
    this._currentProject.set(null);
    this._projects.set([]);
    this._projectConfig.set(null);
    this._error.set(null);
    this._isLoading.set(false);
  }
}
