import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { map } from 'rxjs/operators';
import { ApiService } from '../../core/services/api.service';
import { 
  Package, 
  PackageIdent, 
  PackageSearch, 
  PackageSearchResult,
  PackageSummary,
  LatestPackage
} from '../models/package.model';

@Injectable({
  providedIn: 'root'
})
export class PackageService {
  constructor(
    private apiService: ApiService,
    private http: HttpClient
  ) {}

  /**
   * Gets a package by its identifier
   */
  getPackage(ident: PackageIdent, target?: string): Observable<Package> {
    const { origin, name, version, release } = ident;
    
    let path = `/v1/depot/pkgs/${origin}/${name}`;
    if (version) {
      path += `/${version}`;
      if (release) {
        path += `/${release}`;
      }
    }

    let params = new HttpParams();
    if (target) {
      params = params.set('target', target);
    }

    return this.apiService.get<Package>(path, params);
  }

  /**
   * Gets the latest version of a package
   */
  getLatestPackage(origin: string, name: string, target?: string, channel: string = 'stable'): Observable<Package> {
    let params = new HttpParams().set('channel', channel);
    
    if (target) {
      params = params.set('target', target);
    }

    return this.apiService.get<Package>(`/v1/depot/pkgs/${origin}/${name}/latest`, params);
  }

  /**
   * Searches for packages
   */
  searchPackages(search: PackageSearch): Observable<PackageSearchResult> {
    let params = new HttpParams();
    
    if (search.origin) {
      params = params.set('origin', search.origin);
    }
    
    if (search.query) {
      params = params.set('query', search.query);
    }
    
    if (search.page) {
      params = params.set('page', search.page.toString());
    }
    
    if (search.limit) {
      params = params.set('limit', search.limit.toString());
    }

    if (search.target) {
      params = params.set('target', search.target);
    }

    return this.apiService.get<PackageSearchResult>('/v1/depot/pkgs/search', params);
  }

  /**
   * Lists packages for an origin
   */
  listPackages(origin: string, options: { name?: string, page?: number, limit?: number } = {}): Observable<PackageSearchResult> {
    let path = `/v1/depot/pkgs/${origin}`;
    
    if (options.name) {
      path += `/${options.name}`;
    }
    
    let params = new HttpParams();
    
    if (options.page) {
      params = params.set('page', options.page.toString());
    }
    
    if (options.limit) {
      params = params.set('limit', options.limit.toString());
    }

    return this.apiService.get<PackageSearchResult>(path, params);
  }

  /**
   * Lists package versions
   */
  listVersions(origin: string, name: string): Observable<string[]> {
    return this.apiService.get<any>(`/v1/depot/pkgs/${origin}/${name}/versions`)
      .pipe(map(result => result.versions));
  }

  /**
   * Downloads a package
   */
  downloadPackage(ident: PackageIdent, target?: string): Observable<Blob> {
    const { origin, name, version, release } = ident;
    
    let path = `/v1/depot/pkgs/${origin}/${name}`;
    if (version) {
      path += `/${version}`;
      if (release) {
        path += `/${release}`;
      }
    }
    path += '/download';

    let params = new HttpParams();
    if (target) {
      params = params.set('target', target);
    }

    return this.http.get(this.apiService.getUrl(path, params), {
      responseType: 'blob'
    });
  }

  /**
   * Updates package visibility
   */
  updateVisibility(ident: PackageIdent, visibility: string): Observable<any> {
    const { origin, name, version, release } = ident;
    
    let path = `/v1/depot/pkgs/${origin}/${name}`;
    if (version) {
      path += `/${version}`;
      if (release) {
        path += `/${release}`;
      }
    }
    
    return this.apiService.put<any>(`${path}/visibility/${visibility}`, {});
  }

  /**
   * Lists channels for a package
   */
  getPackageChannels(ident: PackageIdent): Observable<string[]> {
    const { origin, name, version, release } = ident;
    
    const path = `/v1/depot/pkgs/${origin}/${name}/${version}/${release}/channels`;
    
    return this.apiService.get<any>(path)
      .pipe(map(result => result.channels));
  }

  /**
   * Promotes a package to a channel
   */
  promoteToChannel(ident: PackageIdent, channel: string): Observable<any> {
    const { origin, name, version, release } = ident;
    
    return this.apiService.put<any>(
      `/v1/depot/channels/${origin}/${channel}/pkgs/${name}/${version}/${release}/promote`,
      {}
    );
  }

  /**
   * Demotes a package from a channel
   */
  demoteFromChannel(ident: PackageIdent, channel: string): Observable<any> {
    const { origin, name, version, release } = ident;
    
    return this.apiService.put<any>(
      `/v1/depot/channels/${origin}/${channel}/pkgs/${name}/${version}/${release}/demote`,
      {}
    );
  }
}
