import { Injectable, signal, computed, Signal, effect } from '@angular/core';
import { Task, TaskStatus, TaskSearchResult, TaskLog, TaskSearch } from '../models/task.model';
import { TaskService } from '../services/task.service';
import { Observable, catchError, of, tap } from 'rxjs';

@Injectable({
  providedIn: 'root'
})
export class TaskState {
  // Private signals
  private readonly _currentTask = signal<Task | null>(null);
  private readonly _currentTaskLog = signal<TaskLog | null>(null);
  private readonly _taskList = signal<Task[]>([]);
  private readonly _pendingTasks = signal<Task[]>([]);
  private readonly _runningTasks = signal<Task[]>([]);
  private readonly _completedTasks = signal<Task[]>([]);
  private readonly _failedTasks = signal<Task[]>([]);
  private readonly _canceledTasks = signal<Task[]>([]);
  private readonly _totalTasks = signal<number>(0);
  private readonly _loading = signal<boolean>(false);
  private readonly _error = signal<string | null>(null);
  
  // Public readonly signals
  public readonly currentTask = this._currentTask.asReadonly();
  public readonly currentTaskLog = this._currentTaskLog.asReadonly();
  public readonly taskList = this._taskList.asReadonly();
  public readonly pendingTasks = this._pendingTasks.asReadonly();
  public readonly runningTasks = this._runningTasks.asReadonly();
  public readonly completedTasks = this._completedTasks.asReadonly();
  public readonly failedTasks = this._failedTasks.asReadonly();
  public readonly canceledTasks = this._canceledTasks.asReadonly();
  public readonly totalTasks = this._totalTasks.asReadonly();
  public readonly loading = this._loading.asReadonly();
  public readonly error = this._error.asReadonly();
  
  // Computed signals
  public readonly pendingTasksCount: Signal<number> = computed(() => this._pendingTasks().length);
  public readonly runningTasksCount: Signal<number> = computed(() => this._runningTasks().length);
  public readonly completedTasksCount: Signal<number> = computed(() => this._completedTasks().length);
  public readonly failedTasksCount: Signal<number> = computed(() => this._failedTasks().length);
  public readonly canceledTasksCount: Signal<number> = computed(() => this._canceledTasks().length);
  public readonly hasCurrentTask: Signal<boolean> = computed(() => this._currentTask() !== null);
  
  constructor(private taskService: TaskService) {
    // Set up effect to categorize tasks by status whenever the task list changes
    effect(() => {
      const tasks = this._taskList();
      
      // Reset task lists
      const pendingTasks: Task[] = [];
      const runningTasks: Task[] = [];
      const completedTasks: Task[] = [];
      const failedTasks: Task[] = [];
      const canceledTasks: Task[] = [];
      
      // Categorize tasks by status
      tasks.forEach(task => {
        switch (task.status) {
          case TaskStatus.Pending:
            pendingTasks.push(task);
            break;
          case TaskStatus.Running:
            runningTasks.push(task);
            break;
          case TaskStatus.Complete:
            completedTasks.push(task);
            break;
          case TaskStatus.Failed:
            failedTasks.push(task);
            break;
          case TaskStatus.Canceled:
            canceledTasks.push(task);
            break;
        }
      });
      
      // Update signals
      this._pendingTasks.set(pendingTasks);
      this._runningTasks.set(runningTasks);
      this._completedTasks.set(completedTasks);
      this._failedTasks.set(failedTasks);
      this._canceledTasks.set(canceledTasks);
    });
  }
  
  /**
   * Load a task by ID
   */
  loadTask(id: string): Observable<Task | null> {
    this._loading.set(true);
    this._error.set(null);
    
    return this.taskService.getTask(id).pipe(
      tap(task => {
        this._currentTask.set(task);
        this._loading.set(false);
      }),
      catchError(error => {
        this._error.set(`Failed to load task: ${error.message || 'Unknown error'}`);
        this._loading.set(false);
        return of(null);
      })
    );
  }
  
  /**
   * Load task logs
   */
  loadTaskLogs(id: string): Observable<TaskLog | null> {
    this._loading.set(true);
    this._error.set(null);
    
    return this.taskService.getTaskLogs(id).pipe(
      tap(logs => {
        this._currentTaskLog.set(logs);
        this._loading.set(false);
      }),
      catchError(error => {
        this._error.set(`Failed to load task logs: ${error.message || 'Unknown error'}`);
        this._loading.set(false);
        return of(null);
      })
    );
  }
  
  /**
   * Load tasks with search parameters
   */
  loadTasks(params: TaskSearch = {}): Observable<TaskSearchResult | null> {
    this._loading.set(true);
    this._error.set(null);
    
    return this.taskService.searchTasks(params).pipe(
      tap(result => {
        this._taskList.set(result.tasks);
        this._totalTasks.set(result.total);
        this._loading.set(false);
      }),
      catchError(error => {
        this._error.set(`Failed to load tasks: ${error.message || 'Unknown error'}`);
        this._loading.set(false);
        return of(null);
      })
    );
  }
  
  /**
   * Start a task
   */
  startTask(id: string): Observable<Task | null> {
    this._loading.set(true);
    this._error.set(null);
    
    return this.taskService.startTask(id).pipe(
      tap(task => {
        this.updateTaskInList(task);
        if (this._currentTask()?.id === task.id) {
          this._currentTask.set(task);
        }
        this._loading.set(false);
      }),
      catchError(error => {
        this._error.set(`Failed to start task: ${error.message || 'Unknown error'}`);
        this._loading.set(false);
        return of(null);
      })
    );
  }
  
  /**
   * Cancel a task
   */
  cancelTask(id: string): Observable<Task | null> {
    this._loading.set(true);
    this._error.set(null);
    
    return this.taskService.cancelTask(id).pipe(
      tap(task => {
        this.updateTaskInList(task);
        if (this._currentTask()?.id === task.id) {
          this._currentTask.set(task);
        }
        this._loading.set(false);
      }),
      catchError(error => {
        this._error.set(`Failed to cancel task: ${error.message || 'Unknown error'}`);
        this._loading.set(false);
        return of(null);
      })
    );
  }
  
  /**
   * Complete a task
   */
  completeTask(id: string, success: boolean = true): Observable<Task | null> {
    this._loading.set(true);
    this._error.set(null);
    
    return this.taskService.completeTask(id, success).pipe(
      tap(task => {
        this.updateTaskInList(task);
        if (this._currentTask()?.id === task.id) {
          this._currentTask.set(task);
        }
        this._loading.set(false);
      }),
      catchError(error => {
        this._error.set(`Failed to complete task: ${error.message || 'Unknown error'}`);
        this._loading.set(false);
        return of(null);
      })
    );
  }
  
  /**
   * Create a new task
   */
  createTask(task: any): Observable<Task | null> {
    this._loading.set(true);
    this._error.set(null);
    
    return this.taskService.createTask(task).pipe(
      tap(newTask => {
        this._taskList.update(tasks => [...tasks, newTask]);
        this._loading.set(false);
      }),
      catchError(error => {
        this._error.set(`Failed to create task: ${error.message || 'Unknown error'}`);
        this._loading.set(false);
        return of(null);
      })
    );
  }
  
  /**
   * Update a task in the task list
   */
  private updateTaskInList(task: Task): void {
    this._taskList.update(tasks => 
      tasks.map(t => t.id === task.id ? task : t)
    );
  }
  
  /**
   * Clear the current task
   */
  clearCurrentTask(): void {
    this._currentTask.set(null);
    this._currentTaskLog.set(null);
  }
  
  /**
   * Clear all tasks
   */
  clearTasks(): void {
    this._taskList.set([]);
    this._totalTasks.set(0);
  }
  
  /**
   * Clear error
   */
  clearError(): void {
    this._error.set(null);
  }
}
