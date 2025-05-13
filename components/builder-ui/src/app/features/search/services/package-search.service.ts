import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable, catchError, map, throwError } from 'rxjs';
import { ApiService } from '../../../core/services/api.service';
import { Package } from '../../../shared/models/package.model';

export interface PackageSearchResponse {
  results: Package[];
  totalCount: number;
  nextRange: number;
}

@Injectable({
  providedIn: 'root'
})
export class PackageSearchService {
  private apiService = inject(ApiService);
  private http = inject(HttpClient);
  
  /**
   * Search for packages based on query
   * @param origin The origin to filter by, use '*' for all origins
   * @param query The search query
   * @param range The start index for pagination
   * @param limit The number of items per page
   */
  searchPackages(
    origin: string = 'core', 
    query: string = '', 
    range: number = 0,
    limit: number = 50
  ): Observable<PackageSearchResponse> {
    // Build query parameters
    let params: Record<string, string> = {};
    
    // Add origin filter if not '*' (all origins)
    if (origin !== '*') {
      params['origin'] = origin;
    }
    
    // Add query if provided
    if (query) {
      params['query'] = query;
    }
    
    // Add distinct flag to get only the latest versions
    params['distinct'] = 'true';
    
    // Add sorting parameter
    params['sort'] = 'name_asc'; // Sort alphabetically by package name
    
    return this.apiService.get<PackageSearchResponse>(
      '/v1/depot/pkgs/search', 
      params, 
      {
        params: {
          range: range.toString(),
          limit: limit.toString()
        }
      }
    ).pipe(
      catchError(error => {
        console.error('Error searching packages:', error);
        return throwError(() => new Error('Failed to search packages'));
      })
    );
  }
  
  /**
   * Get all versions of a specific package
   * @param origin Package origin
   * @param name Package name
   * @param range Starting index for pagination
   * @param limit Number of items per page
   */
  getPackageVersions(
    origin: string,
    name: string,
    range: number = 0,
    limit: number = 50
  ): Observable<PackageSearchResponse> {
    return this.apiService.get<PackageSearchResponse>(
      `/v1/depot/pkgs/${origin}/${name}/versions`,
      {},
      {
        params: {
          range: range.toString(),
          limit: limit.toString()
        }
      }
    ).pipe(
      catchError(error => {
        console.error('Error fetching package versions:', error);
        return throwError(() => new Error('Failed to fetch package versions'));
      })
    );
  }
  
  /**
   * Utility function to create a package identifier string
   */
  packageString(pkg: Partial<Package>): string {
    return ['origin', 'name', 'version', 'release']
      .map(part => pkg[part as keyof Package])
      .filter(part => part)
      .join('/');
  }
}
