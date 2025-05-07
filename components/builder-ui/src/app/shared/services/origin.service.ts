import { Injectable } from '@angular/core';
import { HttpParams } from '@angular/common/http';
import { Observable, of } from 'rxjs';
import { ApiService } from '../../core/services/api.service';
import { 
  Origin, 
  OriginWithStats,
  OriginMember,
  OriginPublicKey,
  OriginSecretKey,
  OriginKey,
  OriginIntegration,
  OriginInvitation,
  OriginSearch
} from '../models/origin.model';
import { map } from 'rxjs/operators';

@Injectable({
  providedIn: 'root'
})
export class OriginService {
  constructor(private apiService: ApiService) {}

  /**
   * Gets an origin by name
   */
  getOrigin(name: string): Observable<Origin> {
    return this.apiService.get<Origin>(`/v1/depot/origins/${name}`);
  }

  /**
   * Gets origin with additional statistics
   */
  getOriginWithStats(name: string): Observable<OriginWithStats> {
    return this.apiService.get<OriginWithStats>(`/v1/depot/origins/${name}/stats`);
  }

  /**
   * Creates a new origin
   */
  createOrigin(origin: Partial<Origin>): Observable<Origin> {
    return this.apiService.post<Origin>('/v1/depot/origins', origin);
  }

  /**
   * Updates an origin
   */
  updateOrigin(origin: Partial<Origin>): Observable<Origin> {
    return this.apiService.put<Origin>(`/v1/depot/origins/${origin.name}`, origin);
  }

  /**
   * Searches for origins
   */
  searchOrigins(search: OriginSearch = {}): Observable<Origin[]> {
    let params = new HttpParams();
    
    if (search.query) {
      params = params.set('q', search.query);
    }
    
    if (search.page) {
      params = params.set('page', search.page.toString());
    }
    
    if (search.limit) {
      params = params.set('limit', search.limit.toString());
    }
    
    return this.apiService.get<{ origins: Origin[] }>('/v1/depot/origins/search', params)
      .pipe(map(result => result.origins || []));
  }

  /**
   * Lists origins for the current user
   */
  getMyOrigins(): Observable<Origin[]> {
    return this.apiService.get<{ origins: Origin[] }>('/v1/depot/origins')
      .pipe(map(result => result.origins || []));
  }

  /**
   * Lists all origins the user has access to
   */
  getAccessibleOrigins(): Observable<Origin[]> {
    return this.apiService.get<{ origins: Origin[] }>('/v1/depot/origins/accessible')
      .pipe(map(result => result.origins || []));
  }

  /**
   * Gets all members of an origin
   */
  getOriginMembers(originName: string): Observable<OriginMember[]> {
    return this.apiService.get<{ members: OriginMember[] }>(`/v1/depot/origins/${originName}/users`)
      .pipe(map(result => result.members || []));
  }

  /**
   * Invites a user to an origin
   */
  inviteUser(originName: string, username: string, role?: string): Observable<OriginInvitation> {
    const payload = { account: username };
    if (role) {
      Object.assign(payload, { role });
    }
    
    return this.apiService.post<OriginInvitation>(`/v1/depot/origins/${originName}/users/invite`, payload);
  }

  /**
   * Accepts an invitation to join an origin
   */
  acceptInvitation(invitationId: string, token: string): Observable<any> {
    return this.apiService.put<any>(`/v1/depot/origins/invitations/${invitationId}/accept`, { token });
  }

  /**
   * Ignores/rejects an invitation
   */
  ignoreInvitation(invitationId: string): Observable<any> {
    return this.apiService.put<any>(`/v1/depot/origins/invitations/${invitationId}/ignore`, {});
  }

  /**
   * Gets all invitations for a user
   */
  getMyInvitations(): Observable<OriginInvitation[]> {
    return this.apiService.get<{ invitations: OriginInvitation[] }>('/v1/depot/user/invitations')
      .pipe(map(result => result.invitations || []));
  }

  /**
   * Gets all invitations for an origin
   */
  getOriginInvitations(originName: string): Observable<OriginInvitation[]> {
    return this.apiService.get<{ invitations: OriginInvitation[] }>(`/v1/depot/origins/${originName}/invitations`)
      .pipe(map(result => result.invitations || []));
  }

  /**
   * Gets origin secret keys
   */
  getOriginSecretKeys(originName: string): Observable<OriginSecretKey[]> {
    return this.apiService.get<{ secret_keys: OriginSecretKey[] }>(`/v1/depot/origins/${originName}/secret_keys`)
      .pipe(map(result => result.secret_keys || []));
  }

  /**
   * Gets origin public keys
   */
  getOriginPublicKeys(originName: string): Observable<OriginPublicKey[]> {
    return this.apiService.get<{ public_keys: OriginPublicKey[] }>(`/v1/depot/origins/${originName}/keys`)
      .pipe(map(result => result.public_keys || []));
  }

  /**
   * Uploads an origin public key
   */
  uploadPublicKey(originName: string, key: string): Observable<OriginPublicKey> {
    return this.apiService.post<OriginPublicKey>(`/v1/depot/origins/${originName}/keys`, { body: key });
  }

  /**
   * Uploads an origin secret key
   */
  uploadSecretKey(originName: string, key: string): Observable<OriginSecretKey> {
    return this.apiService.post<OriginSecretKey>(`/v1/depot/origins/${originName}/secret_keys`, { body: key });
  }

  /**
   * Download a public key
   */
  downloadPublicKey(originName: string, revision?: string): Observable<string> {
    let url = `/v1/depot/origins/${originName}/keys`;
    if (revision) {
      url += `/${revision}`;
    }
    
    return this.apiService.getText(url);
  }

  /**
   * Gets origin integrations
   */
  getOriginIntegrations(originName: string, type?: string): Observable<OriginIntegration[]> {
    let url = `/v1/depot/origins/${originName}/integrations`;
    if (type) {
      url += `/${type}`;
    }
    
    return this.apiService.get<{ integrations: OriginIntegration[] }>(url)
      .pipe(map(result => result.integrations || []));
  }

  /**
   * Creates an origin integration
   */
  createIntegration(originName: string, type: string, integration: any): Observable<OriginIntegration> {
    return this.apiService.post<OriginIntegration>(
      `/v1/depot/origins/${originName}/integrations/${type}`,
      integration
    );
  }

  /**
   * Deletes an origin integration
   */
  deleteIntegration(originName: string, type: string, name: string): Observable<any> {
    return this.apiService.delete(`/v1/depot/origins/${originName}/integrations/${type}/${name}`);
  }
}
