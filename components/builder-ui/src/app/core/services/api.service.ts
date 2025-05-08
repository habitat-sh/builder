import { Injectable } from '@angular/core';
import { HttpClient, HttpParams, HttpHeaders } from '@angular/common/http';
import { Observable, throwError } from 'rxjs';
import { catchError, timeout, map } from 'rxjs/operators';
import { environment } from '../../../environments/environment';

/**
 * Generic API service for making HTTP requests
 */
@Injectable({
  providedIn: 'root'
})
export class ApiService {
  private apiUrl = environment.apiUrl;
  private defaultTimeout = 30000; // 30 seconds

  constructor(private http: HttpClient) {}

  /**
   * Make a GET request to the API
   */
  get<T>(endpoint: string, params: any = {}, options: any = {}): Observable<T> {
    const url = this.buildUrl(endpoint);
    const httpParams = this.buildParams(params);
    const httpOptions = {
      ...this.buildOptions(options),
      params: httpParams,
      observe: 'body' as const
    };

    // @ts-ignore - Type 'Observable<HttpEvent<T>>' is not assignable to type 'Observable<T>'
    return this.http.get<T>(url, httpOptions).pipe(
      timeout({ first: options.timeout || this.defaultTimeout }),
      catchError(error => this.handleError(error))
    );
  }

  /**
   * Make a POST request to the API
   */
  post<T>(endpoint: string, body: any, options: any = {}): Observable<T> {
    const url = this.buildUrl(endpoint);
    const httpOptions = {
      ...this.buildOptions(options),
      observe: 'body' as const
    };

    // @ts-ignore - Type 'Observable<HttpEvent<T>>' is not assignable to type 'Observable<T>'
    return this.http.post<T>(url, body, httpOptions).pipe(
      timeout({ first: options.timeout || this.defaultTimeout }),
      catchError(error => this.handleError(error))
    );
  }

  /**
   * Make a PUT request to the API
   */
  put<T>(endpoint: string, body: any, options: any = {}): Observable<T> {
    const url = this.buildUrl(endpoint);
    const httpOptions = {
      ...this.buildOptions(options),
      observe: 'body' as const
    };

    // @ts-ignore - Type 'Observable<HttpEvent<T>>' is not assignable to type 'Observable<T>'
    return this.http.put<T>(url, body, httpOptions).pipe(
      timeout({ first: options.timeout || this.defaultTimeout }),
      catchError(error => this.handleError(error))
    );
  }

  /**
   * Make a PATCH request to the API
   */
  patch<T>(endpoint: string, body: any, options: any = {}): Observable<T> {
    const url = this.buildUrl(endpoint);
    const httpOptions = {
      ...this.buildOptions(options),
      observe: 'body' as const
    };

    // @ts-ignore - Type 'Observable<HttpEvent<T>>' is not assignable to type 'Observable<T>'
    return this.http.patch<T>(url, body, httpOptions).pipe(
      timeout({ first: options.timeout || this.defaultTimeout }),
      catchError(error => this.handleError(error))
    );
  }

  /**
   * Make a DELETE request to the API
   */
  delete<T>(endpoint: string, options: any = {}): Observable<T> {
    const url = this.buildUrl(endpoint);
    const httpOptions = {
      ...this.buildOptions(options),
      observe: 'body' as const
    };

    // @ts-ignore - Type 'Observable<HttpEvent<T>>' is not assignable to type 'Observable<T>'
    return this.http.delete<T>(url, httpOptions).pipe(
      timeout({ first: options.timeout || this.defaultTimeout }),
      catchError(error => this.handleError(error))
    );
  }

  /**
   * Get text content from a URL
   */
  getText(url: string, options: any = {}): Observable<string> {
    return this.http.get(url, {
      headers: new HttpHeaders({
        'Content-Type': 'text/plain',
        ...options.headers || {}
      }),
      responseType: 'text',
      observe: 'body' as const
    }).pipe(
      timeout({ first: options.timeout || this.defaultTimeout }),
      catchError(error => this.handleError(error))
    );
  }

  /**
   * Build the full URL for an API endpoint
   */
  private buildUrl(endpoint: string): string {
    // Remove leading slash if present
    endpoint = endpoint.startsWith('/') ? endpoint.slice(1) : endpoint;
    return `${this.apiUrl}/${endpoint}`;
  }
  
  /**
   * Get a fully qualified URL for an API endpoint
   * 
   * @param endpoint API endpoint path
   * @param params Optional query parameters
   * @returns The fully qualified URL as a string
   */
  getUrl(endpoint: string, params?: HttpParams): string {
    const url = this.buildUrl(endpoint);
    
    if (params && params.keys().length > 0) {
      return `${url}?${params.toString()}`;
    }
    
    return url;
  }

  /**
   * Convert a params object to HttpParams
   */
  private buildParams(params: any): HttpParams {
    let httpParams = new HttpParams();
    
    if (params) {
      Object.keys(params).forEach(key => {
        if (params[key] !== undefined && params[key] !== null) {
          httpParams = httpParams.set(key, params[key].toString());
        }
      });
    }
    
    return httpParams;
  }

  /**
   * Build HTTP options object with headers and params
   */
  private buildOptions(options: any): any {
    const httpOptions: any = {
      headers: new HttpHeaders({
        'Content-Type': 'application/json',
        ...options.headers || {}
      })
    };

    if (options.params) {
      httpOptions.params = this.buildParams(options.params);
    }

    return httpOptions;
  }

  /**
   * Handle HTTP error responses
   */
  private handleError(error: any): Observable<never> {
    let errorMessage = 'An unknown error occurred';
    
    if (error.error instanceof ErrorEvent) {
      // Client-side error
      errorMessage = `Error: ${error.error.message}`;
    } else {
      // Server-side error
      errorMessage = `Error Code: ${error.status}\nMessage: ${error.message}`;
    }
    
    return throwError(() => ({
      error,
      message: errorMessage,
      status: error.status || 500
    }));
  }
}
