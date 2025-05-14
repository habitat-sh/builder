import { Injectable, inject, signal, computed } from '@angular/core';
import { ApiService } from '../../../core/services/api.service';
import { AuthService } from '../../../core/services/auth.service';
import { Observable, map, tap, catchError, of, throwError } from 'rxjs';
import { Origin, OriginInvitation } from '../models/origin.model';
import { HttpErrorResponse } from '@angular/common/http';

@Injectable({
  providedIn: 'root'
})
export class OriginService {
  private api = inject(ApiService);
  private authService = inject(AuthService);

  // State management with signals
  private _origins = signal<Origin[]>([]);
  private _invitations = signal<OriginInvitation[]>([]);
  private _loading = signal<boolean>(false);
  private _error = signal<string | null>(null);

  // Public readonly signals
  readonly origins = this._origins.asReadonly();
  readonly invitations = this._invitations.asReadonly();
  readonly loading = this._loading.asReadonly();
  readonly error = this._error.asReadonly();

  // Computed values
  readonly allOriginItems = computed(() => {
    const myOrigins = this._origins();
    const myInvitations = this._invitations().map(i => ({
      ...i,
      isInvite: true
    }));
    
    return [...myOrigins, ...myInvitations].sort((a, b) => {
      const nameA = 'name' in a ? a.name : a.origin;
      const nameB = 'name' in b ? b.name : b.origin;
      return nameA.localeCompare(nameB);
    });
  });

  /**
   * Fetch all origins owned by or accessible to the current user
   */
  fetchMyOrigins(): Observable<Origin[]> {
    this._loading.set(true);
    this._error.set(null);
    
    return this.api.get<Origin[]>('/api/v1/user/origins').pipe(
      tap(origins => {
        this._origins.set(origins);
        this._loading.set(false);
      }),
      catchError((error: HttpErrorResponse) => {
        this._error.set(error.message || 'Failed to load origins');
        this._loading.set(false);
        return of([]);
      })
    );
  }

  /**
   * Fetch all origin invitations for the current user
   */
  fetchMyInvitations(): Observable<OriginInvitation[]> {
    this._loading.set(true);
    this._error.set(null);
    
    return this.api.get<OriginInvitation[]>('/api/v1/user/invitations').pipe(
      tap(invitations => {
        this._invitations.set(invitations);
        this._loading.set(false);
      }),
      catchError((error: HttpErrorResponse) => {
        this._error.set(error.message || 'Failed to load invitations');
        this._loading.set(false);
        return of([]);
      })
    );
  }

  /**
   * Accept an invitation to join an origin
   */
  acceptInvitation(invitationId: string, originName: string): Observable<any> {
    return this.api.put(`/api/v1/invitations/${invitationId}`, {}).pipe(
      tap(() => {
        // Remove the invitation from the list
        const updatedInvitations = this._invitations().filter(i => i.id !== invitationId);
        this._invitations.set(updatedInvitations);
        
        // Refresh the origins list
        this.fetchMyOrigins().subscribe();
      }),
      catchError((error: HttpErrorResponse) => {
        this._error.set(error.message || `Failed to accept invitation for ${originName}`);
        return of(null);
      })
    );
  }

  /**
   * Ignore/decline an invitation to join an origin
   */
  ignoreInvitation(invitationId: string, originName: string): Observable<any> {
    return this.api.delete(`/api/v1/invitations/${invitationId}`).pipe(
      tap(() => {
        // Remove the invitation from the list
        const updatedInvitations = this._invitations().filter(i => i.id !== invitationId);
        this._invitations.set(updatedInvitations);
      }),
      catchError((error: HttpErrorResponse) => {
        this._error.set(error.message || `Failed to ignore invitation for ${originName}`);
        return of(null);
      })
    );
  }

  /**
   * Initialize the service by fetching both origins and invitations
   */
  initialize() {
    if (this.authService.isAuthenticated()) {
      this.fetchMyOrigins().subscribe();
      this.fetchMyInvitations().subscribe();
    }
  }

  /**
   * Create a new origin
   */
  createOrigin(originData: Partial<Origin>): Observable<Origin> {
    this._loading.set(true);
    this._error.set(null);
    
    return this.api.post<Origin>('/api/v1/origins', originData).pipe(
      tap(origin => {
        // Add the new origin to the list
        const updatedOrigins = [...this._origins(), origin];
        this._origins.set(updatedOrigins);
        this._loading.set(false);
      }),
      catchError((error: HttpErrorResponse) => {
        this._error.set(error.message || 'Failed to create origin');
        this._loading.set(false);
        return throwError(() => new Error(error.message || 'Failed to create origin'));
      })
    );
  }

  /**
   * Helper to check if an item is an invitation
   */
  isInvitation(item: Origin | OriginInvitation): boolean {
    return 'isInvite' in item && item.isInvite === true;
  }

  /**
   * Get the visibility icon for an origin
   */
  getVisibilityIcon(item: Origin | OriginInvitation): string {
    if ('default_package_visibility' in item) {
      return item.default_package_visibility === 'private' ? 'lock' : 'public';
    }
    return 'unknown';
  }

  /**
   * Get the visibility label for an origin
   */
  getVisibilityLabel(item: Origin | OriginInvitation): string {
    if ('default_package_visibility' in item) {
      return item.default_package_visibility === 'private' ? 'Private' : 'Public';
    }
    return 'Unknown';
  }

  /**
   * Get the name of an origin item (either origin or invitation)
   */
  getName(item: Origin | OriginInvitation): string {
    return 'name' in item ? item.name : item.origin;
  }

  /**
   * Get the package count for an origin item
   */
  getPackageCount(item: Origin | OriginInvitation): string {
    if ('package_count' in item) {
      return item.package_count !== undefined ? `${item.package_count}` : '-';
    }
    return '-';
  }
}
