import { Injectable } from '@angular/core';
import { HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { map } from 'rxjs/operators';
import { ApiService } from '../../core/services/api.service';
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
export class BuildService {
  constructor(private apiService: ApiService) {}

  /**
   * Gets a build job
   */
  getBuildJob(id: string): Observable<BuildJob> {
    return this.apiService.get<BuildJob>(`/v1/jobs/${id}`);
  }

  /**
   * Gets build logs
   */
  getBuildLogs(id: string): Observable<BuildLog> {
    return this.apiService.get<BuildLog>(`/v1/jobs/${id}/log`);
  }

  /**
   * Schedules a build for a package
   */
  scheduleBuild(origin: string, name: string, target?: string): Observable<BuildJob> {
    let payload: any = {};
    
    if (target) {
      payload.target = target;
    }
    
    return this.apiService.post<BuildJob>(`/v1/jobs/${origin}/${name}/schedule`, payload);
  }

  /**
   * Cancels a build
   */
  cancelBuild(id: string): Observable<any> {
    return this.apiService.delete(`/v1/jobs/${id}/cancel`);
  }

  /**
   * Searches for builds
   */
  searchBuilds(search: BuildSearch = {}): Observable<BuildJob[]> {
    let params = new HttpParams();
    
    if (search.origin) {
      params = params.set('origin', search.origin);
    }
    
    if (search.name) {
      params = params.set('package', search.name);
    }
    
    if (search.page) {
      params = params.set('page', search.page.toString());
    }
    
    if (search.limit) {
      params = params.set('limit', search.limit.toString());
    }
    
    if (search.status && search.status.length > 0) {
      search.status.forEach(status => {
        params = params.append('status', status);
      });
    }
    
    if (search.fromDate) {
      params = params.set('from_date', search.fromDate.toISOString());
    }
    
    if (search.toDate) {
      params = params.set('to_date', search.toDate.toISOString());
    }
    
    return this.apiService.get<{ jobs: BuildJob[] }>('/v1/jobs/search', params)
      .pipe(map(result => result.jobs || []));
  }

  /**
   * Gets projects for an origin
   */
  getProjects(origin: string, search: ProjectSearch = {}): Observable<Project[]> {
    let params = new HttpParams();
    
    if (search.name) {
      params = params.set('name', search.name);
    }
    
    if (search.page) {
      params = params.set('page', search.page.toString());
    }
    
    if (search.limit) {
      params = params.set('limit', search.limit.toString());
    }
    
    return this.apiService.get<{ projects: Project[] }>(`/v1/projects/${origin}`, params)
      .pipe(map(result => result.projects || []));
  }

  /**
   * Gets a project
   */
  getProject(origin: string, name: string): Observable<Project> {
    return this.apiService.get<Project>(`/v1/projects/${origin}/${name}`);
  }

  /**
   * Creates a project
   */
  createProject(project: Partial<Project>): Observable<Project> {
    return this.apiService.post<Project>(`/v1/projects/${project.originName}`, project);
  }

  /**
   * Updates a project
   */
  updateProject(project: Partial<Project>): Observable<Project> {
    return this.apiService.put<Project>(
      `/v1/projects/${project.originName}/${project.name}`,
      project
    );
  }

  /**
   * Deletes a project
   */
  deleteProject(origin: string, name: string): Observable<any> {
    return this.apiService.delete(`/v1/projects/${origin}/${name}`);
  }

  /**
   * Gets a project's build configuration
   */
  getProjectConfig(origin: string, name: string): Observable<ProjectConfig> {
    return this.apiService.get<{ config: ProjectConfig }>(`/v1/projects/${origin}/${name}/config`)
      .pipe(map(result => result.config));
  }

  /**
   * Updates a project's build configuration
   */
  updateProjectConfig(origin: string, name: string, config: Partial<ProjectConfig>): Observable<ProjectConfig> {
    return this.apiService.put<ProjectConfig>(`/v1/projects/${origin}/${name}/config`, config);
  }
}
