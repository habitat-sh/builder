import { Injectable, signal } from '@angular/core';

/**
 * Service for managing loading state throughout the application
 */
@Injectable({
  providedIn: 'root'
})
export class LoadingService {
  // State signals
  private _isLoading = signal(false);
  private _message = signal<string | undefined>(undefined);
  private _loaderCount = 0;

  // Exposed readable signals
  public readonly isLoading = this._isLoading.asReadonly();
  public readonly message = this._message.asReadonly();

  /**
   * Start loading with an optional message
   * 
   * @param message Optional message to display
   */
  start(message?: string): void {
    this._loaderCount++;
    this._isLoading.set(true);
    this._message.set(message);
  }

  /**
   * Stop loading
   */
  stop(): void {
    this._loaderCount = Math.max(0, this._loaderCount - 1);
    
    if (this._loaderCount === 0) {
      this._isLoading.set(false);
      this._message.set(undefined);
    }
  }

  /**
   * Update the loading message
   * 
   * @param message New message to display
   */
  updateMessage(message: string): void {
    this._message.set(message);
  }

  /**
   * Reset loading state completely
   */
  reset(): void {
    this._loaderCount = 0;
    this._isLoading.set(false);
    this._message.set(undefined);
  }
}
