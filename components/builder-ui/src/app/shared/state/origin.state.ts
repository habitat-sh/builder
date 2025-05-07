import { Injectable, Signal, computed, signal } from '@angular/core';
import { EMPTY, catchError, tap } from 'rxjs';
import { NotificationService } from '../../core/services/notification.service';
import { LoadingService } from '../../core/services/loading.service';
import { OriginService } from '../services/origin.service';
import {
  Origin,
  OriginIntegration,
  OriginInvitation,
  OriginKey,
  OriginMember,
  OriginPublicKey,
  OriginSecretKey,
  OriginWithStats
} from '../models/origin.model';

@Injectable({
  providedIn: 'root'
})
export class OriginState {
  // State signals
  private readonly _currentOrigin = signal<Origin | null>(null);
  private readonly _originWithStats = signal<OriginWithStats | null>(null);
  private readonly _originMembers = signal<OriginMember[]>([]);
  private readonly _originInvitations = signal<OriginInvitation[]>([]);
  private readonly _myOrigins = signal<Origin[]>([]);
  private readonly _accessibleOrigins = signal<Origin[]>([]);
  private readonly _originPublicKeys = signal<OriginPublicKey[]>([]);
  private readonly _originSecretKeys = signal<OriginSecretKey[]>([]);
  private readonly _originIntegrations = signal<OriginIntegration[]>([]);
  private readonly _error = signal<string | null>(null);
  private readonly _isLoading = signal<boolean>(false);

  // Public readable signals
  public readonly currentOrigin = this._currentOrigin.asReadonly();
  public readonly originWithStats = this._originWithStats.asReadonly();
  public readonly originMembers = this._originMembers.asReadonly();
  public readonly originInvitations = this._originInvitations.asReadonly();
  public readonly myOrigins = this._myOrigins.asReadonly();
  public readonly accessibleOrigins = this._accessibleOrigins.asReadonly();
  public readonly originPublicKeys = this._originPublicKeys.asReadonly();
  public readonly originSecretKeys = this._originSecretKeys.asReadonly();
  public readonly originIntegrations = this._originIntegrations.asReadonly();
  public readonly error = this._error.asReadonly();
  public readonly isLoading = this._isLoading.asReadonly();

  // Computed signals
  public readonly hasAdminAccess: Signal<boolean> = computed(() => {
    if (!this._currentOrigin()) {
      return false;
    }
    
    return this._originMembers().some(member => 
      ['owner', 'administrator'].includes(member.role));
  });

  public readonly hasKeys: Signal<boolean> = computed(() => 
    this._originPublicKeys().length > 0);

  constructor(
    private originService: OriginService,
    private loadingService: LoadingService,
    private notificationService: NotificationService
  ) {}

  /**
   * Loads an origin by name
   */
  loadOrigin(name: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.originService.getOrigin(name)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load origin');
          this.notificationService.error('Failed to load origin', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(origin => {
        this._currentOrigin.set(origin);
        this._isLoading.set(false);
      });
  }

  /**
   * Loads origin with statistics
   */
  loadOriginWithStats(name: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.originService.getOriginWithStats(name)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load origin statistics');
          this.notificationService.error('Failed to load origin statistics', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(originWithStats => {
        this._originWithStats.set(originWithStats);
        this._isLoading.set(false);
      });
  }

  /**
   * Loads origin members
   */
  loadOriginMembers(name: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.originService.getOriginMembers(name)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load origin members');
          this.notificationService.error('Failed to load origin members', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(members => {
        this._originMembers.set(members);
        this._isLoading.set(false);
      });
  }

  /**
   * Loads origin invitations
   */
  loadOriginInvitations(name: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.originService.getOriginInvitations(name)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load origin invitations');
          this.notificationService.error('Failed to load origin invitations', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(invitations => {
        this._originInvitations.set(invitations);
        this._isLoading.set(false);
      });
  }

  /**
   * Loads origins for the current user
   */
  loadMyOrigins(): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.originService.getMyOrigins()
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load your origins');
          this.notificationService.error('Failed to load your origins', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(origins => {
        this._myOrigins.set(origins);
        this._isLoading.set(false);
      });
  }

  /**
   * Loads origins accessible to the current user
   */
  loadAccessibleOrigins(): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.originService.getAccessibleOrigins()
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load accessible origins');
          this.notificationService.error('Failed to load accessible origins', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(origins => {
        this._accessibleOrigins.set(origins);
        this._isLoading.set(false);
      });
  }

  /**
   * Loads origin public keys
   */
  loadOriginPublicKeys(name: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.originService.getOriginPublicKeys(name)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load origin public keys');
          this.notificationService.error('Failed to load origin public keys', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(keys => {
        this._originPublicKeys.set(keys);
        this._isLoading.set(false);
      });
  }

  /**
   * Loads origin secret keys
   */
  loadOriginSecretKeys(name: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.originService.getOriginSecretKeys(name)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load origin secret keys');
          this.notificationService.error('Failed to load origin secret keys', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(keys => {
        this._originSecretKeys.set(keys);
        this._isLoading.set(false);
      });
  }

  /**
   * Loads origin integrations
   */
  loadOriginIntegrations(name: string, type?: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.originService.getOriginIntegrations(name, type)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to load origin integrations');
          this.notificationService.error('Failed to load origin integrations', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(integrations => {
        this._originIntegrations.set(integrations);
        this._isLoading.set(false);
      });
  }

  /**
   * Creates a new origin
   */
  createOrigin(origin: Partial<Origin>): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.originService.createOrigin(origin)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to create origin');
          this.notificationService.error('Failed to create origin', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(newOrigin => {
        this._currentOrigin.set(newOrigin);
        this._isLoading.set(false);
        this.notificationService.success(`Origin ${newOrigin.name} created successfully`);
        
        // Refresh my origins list
        this.loadMyOrigins();
      });
  }

  /**
   * Invites a user to an origin
   */
  inviteUser(originName: string, username: string, role?: string): void {
    this._isLoading.set(true);
    this._error.set(null);

    this.originService.inviteUser(originName, username, role)
      .pipe(
        tap(() => this.loadingService.start()),
        catchError(err => {
          this._error.set(err.message || 'Failed to invite user');
          this.notificationService.error('Failed to invite user', err.message);
          this._isLoading.set(false);
          this.loadingService.stop();
          return EMPTY;
        }),
        tap(() => this.loadingService.stop())
      )
      .subscribe(() => {
        this._isLoading.set(false);
        this.notificationService.success(`${username} was invited to ${originName}`);
        
        // Refresh invitations
        this.loadOriginInvitations(originName);
      });
  }

  /**
   * Resets the state
   */
  reset(): void {
    this._currentOrigin.set(null);
    this._originWithStats.set(null);
    this._originMembers.set([]);
    this._originInvitations.set([]);
    this._myOrigins.set([]);
    this._accessibleOrigins.set([]);
    this._originPublicKeys.set([]);
    this._originSecretKeys.set([]);
    this._originIntegrations.set([]);
    this._error.set(null);
    this._isLoading.set(false);
  }
}
