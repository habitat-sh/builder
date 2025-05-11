import { NgModule, Optional, SkipSelf } from '@angular/core';
import { HTTP_INTERCEPTORS } from '@angular/common/http';
import { environment } from '../../../environments/environment';

import { MockAuthInterceptor } from './mock-auth.interceptor';
import { MockPackageApiInterceptor } from './mock-package-api.interceptor';
import { MockAuthService } from './mock-auth.service';
import { AuthService } from '../services/auth.service';

/**
 * This module provides mock services for development/testing.
 * It should only be imported in the root module.
 */
@NgModule({
  providers: [
    // Only provide mock interceptors if mocks are enabled in environment
    ...(environment.useMocks ? [
      {
        provide: HTTP_INTERCEPTORS,
        useClass: MockAuthInterceptor,
        multi: true
      },
      {
        provide: HTTP_INTERCEPTORS,
        useClass: MockPackageApiInterceptor,
        multi: true
      },
      // Replace the real AuthService with the mock implementation
      {
        provide: AuthService,
        useClass: MockAuthService
      }
    ] : [])
  ]
})
export class MockProvidersModule {
  constructor(@Optional() @SkipSelf() parentModule?: MockProvidersModule) {
    if (parentModule) {
      throw new Error('MockProvidersModule is already loaded. Import it in the AppModule only.');
    }
    
    if (environment.useMocks) {
      console.log('MockProvidersModule: Mock services have been enabled.');
    }
  }
}
