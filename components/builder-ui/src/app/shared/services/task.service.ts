import { Injectable } from '@angular/core';
import { HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { map } from 'rxjs/operators';
import { ApiService } from '../../core/services/api.service';
import {
  Task,
  TaskLog,
  TaskSearch,
  TaskSearchResult,
  NewTaskRequest,
  TaskUpdateRequest,
  TaskStatus
} from '../models/task.model';

@Injectable({
  providedIn: 'root'
})
export class TaskService {
  constructor(private apiService: ApiService) {}

  /**
   * Gets a task by ID
   */
  getTask(id: string): Observable<Task> {
    return this.apiService.get<Task>(`/v1/tasks/${id}`);
  }

  /**
   * Gets task logs
   */
  getTaskLogs(id: string): Observable<TaskLog> {
    return this.apiService.get<TaskLog>(`/v1/tasks/${id}/log`);
  }

  /**
   * Creates a new task
   */
  createTask(task: NewTaskRequest): Observable<Task> {
    return this.apiService.post<Task>('/v1/tasks', task);
  }

  /**
   * Updates a task
   */
  updateTask(id: string, updates: TaskUpdateRequest): Observable<Task> {
    return this.apiService.patch<Task>(`/v1/tasks/${id}`, updates);
  }

  /**
   * Starts a task
   */
  startTask(id: string): Observable<Task> {
    return this.apiService.post<Task>(`/v1/tasks/${id}/start`, {});
  }

  /**
   * Cancels a task
   */
  cancelTask(id: string): Observable<Task> {
    return this.apiService.post<Task>(`/v1/tasks/${id}/cancel`, {});
  }

  /**
   * Completes a task
   */
  completeTask(id: string, success: boolean = true): Observable<Task> {
    return this.apiService.post<Task>(`/v1/tasks/${id}/complete`, { success });
  }

  /**
   * Searches tasks
   */
  searchTasks(params: TaskSearch = {}): Observable<TaskSearchResult> {
    const httpParams: Record<string, any> = { ...params };
    
    return this.apiService.get<TaskSearchResult>('/v1/tasks', httpParams);
  }

  /**
   * Gets tasks by status
   */
  getTasksByStatus(status: TaskStatus, limit: number = 10, offset: number = 0): Observable<TaskSearchResult> {
    return this.searchTasks({
      status,
      limit,
      offset
    });
  }

  /**
   * Gets tasks assigned to a user
   */
  getTasksAssignedToUser(userId: string, limit: number = 10, offset: number = 0): Observable<TaskSearchResult> {
    return this.searchTasks({
      assignedTo: userId,
      limit,
      offset
    });
  }

  /**
   * Gets tasks created by a user
   */
  getTasksCreatedByUser(userId: string, limit: number = 10, offset: number = 0): Observable<TaskSearchResult> {
    return this.searchTasks({
      createdBy: userId,
      limit,
      offset
    });
  }

  /**
   * Gets tasks related to a build
   */
  getTasksForBuild(buildId: string): Observable<Task[]> {
    const params = {
      relatedBuildId: buildId
    };
    
    return this.apiService.get<Task[]>('/v1/tasks/build', params);
  }

  /**
   * Gets tasks related to a package
   */
  getTasksForPackage(origin: string, name: string): Observable<Task[]> {
    const params = {
      origin,
      name
    };
    
    return this.apiService.get<Task[]>('/v1/tasks/package', params);
  }
}
