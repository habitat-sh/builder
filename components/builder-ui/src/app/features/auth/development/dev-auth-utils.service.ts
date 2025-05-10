import { Injectable } from '@angular/core';
import { environment } from '../../../../environments/environment';
import { Router } from '@angular/router';
import { AuthService, User } from '../../../core/services/auth.service';

/**
 * Development utility service for authentication
 * This service provides convenience methods to help with development and testing
 */
@Injectable({
  providedIn: 'root'
})
export class DevAuthUtilsService {
  constructor(
    private authService: AuthService,
    private router: Router
  ) {}

  /**
   * Create a mock authentication session for development
   * @param options Custom options for mock user
   */
  createMockSession(options: Partial<User> = {}) {
    if (!environment.useMocks) {
      console.warn('Mock sessions can only be created when useMocks is enabled in environment');
      return;
    }

    // Create mock user data
    const mockUser: User = {
      id: options.id || 'mock-user-123',
      name: options.name || 'Development User',
      email: options.email || 'dev@example.com',
      avatar: options.avatar || 'https://i.pravatar.cc/150?u=builder',
      role: options.role || 'contributor',
      permissions: options.permissions || ['read:packages', 'write:packages', 'read:origins']
    };

    // Create mock token
    const mockToken = `mock_token_${Math.random().toString(36).substring(2)}`;

    // Calculate expiration (24 hours from now)
    const expiration = new Date();
    expiration.setHours(expiration.getHours() + 24);

    // Access the private method through any type assertion
    // This is only for development and should not be used in production
    (this.authService as any).setAuthState(mockToken, mockUser, expiration);

    console.info('✅ Development mock session created:', {
      user: mockUser,
      token: `${mockToken.substring(0, 10)}...`,
      expires: expiration
    });

    return mockUser;
  }

  /**
   * End the mock session and redirect to sign in
   */
  endMockSession() {
    if (!environment.useMocks) {
      console.warn('Only available in mock mode');
      return;
    }

    this.authService.logout(true);
    console.info('✅ Development mock session ended.');
  }
}
