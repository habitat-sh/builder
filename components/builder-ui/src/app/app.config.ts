import { ApplicationConfig, provideZoneChangeDetection, importProvidersFrom, APP_INITIALIZER } from '@angular/core';
import { provideRouter, withRouterConfig } from '@angular/router';
import { provideHttpClient, withInterceptors } from '@angular/common/http';
import { provideAnimations } from '@angular/platform-browser/animations';
import { Meta, Title, provideClientHydration } from '@angular/platform-browser';
import { environment } from '../environments/environment';

import { routes } from './app.routes';
import { AuthInterceptor } from './core/interceptors/auth.interceptor';
import { ErrorInterceptor } from './core/interceptors/error.interceptor';
import { LoadingInterceptor } from './core/interceptors/loading.interceptor';
import { MockProvidersModule } from './core/mocks/mock-providers.module';

// Services that were previously in CoreModule
import { ApiService } from './core/services/api.service';
import { AuthService } from './core/services/auth.service';
import { NotificationService } from './core/services/notification.service';
import { LoadingService } from './core/services/loading.service';
import { DialogService } from './core/services/dialog.service';

export const appConfig: ApplicationConfig = {
  providers: [
    provideZoneChangeDetection({ eventCoalescing: true }),
    provideRouter(
      routes,
      withRouterConfig({
        onSameUrlNavigation: 'reload',
        paramsInheritanceStrategy: 'always'
      })
    ),
    provideHttpClient(withInterceptors([
      AuthInterceptor,
      ErrorInterceptor,
      LoadingInterceptor
    ])),
    provideAnimations(),
    provideClientHydration(),
    
    // Import mock providers module for development
    ...(environment.useMocks ? [importProvidersFrom(MockProvidersModule)] : []),
    
    // Core services
    ApiService,
    AuthService,
    NotificationService,
    LoadingService,
    DialogService,
    Title,
    Meta,
    
    // Auth initializer to handle token refresh on app startup
    { 
      provide: APP_INITIALIZER,
      useFactory: (authService: AuthService) => {
        return () => {
          console.log('App Initializer: Checking authentication state');
          try {
            console.log('App Initializer: AuthService methods available:', {
              validateAuthState: typeof authService.validateAuthState === 'function',
              isAuthenticated: typeof authService.isAuthenticated === 'function',
              isTokenExpired: typeof authService.isTokenExpired === 'function',
              refreshToken: typeof authService.refreshToken === 'function'
            });
            
            // First ensure the auth state is properly loaded
            authService.validateAuthState();
            
            // Use safe checks for authentication and token expiration
            const isAuthenticated = typeof authService.isAuthenticated === 'function' 
              ? authService.isAuthenticated() 
              : false;
              
            const isExpired = typeof authService.isTokenExpired === 'function' 
              ? authService.isTokenExpired() 
              : false;
            
            // If authenticated but token near expiration, refresh it
            if (isAuthenticated && isExpired) {
              console.log('App Initializer: Token needs refresh');
              return new Promise<boolean>((resolve) => {
                authService.refreshToken().subscribe({
                  next: (result) => resolve(result),
                  error: () => resolve(false)
                });
              });
            }
          } catch (error) {
            console.error('App Initializer: Error checking auth state', error);
          }
          return Promise.resolve(true);
        };
      },
      deps: [AuthService],
      multi: true 
    },
    
    // Import MockProvidersModule conditionally
    ...(environment.useMocks ? [importProvidersFrom(MockProvidersModule)] : [])
  ]
};
