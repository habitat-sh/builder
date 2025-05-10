import { TestBed } from '@angular/core/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { HttpClientTestingModule } from '@angular/common/http/testing';
import { ActivatedRoute, Router, convertToParamMap } from '@angular/router';
import { of } from 'rxjs';

import { OAuthCallbackComponent } from './oauth-callback.component';
import { AuthService } from '../../../core/services/auth.service';

describe('OAuthCallbackComponent', () => {
  let component: OAuthCallbackComponent;
  let router: Router;
  
  // Mock services
  const authServiceMock = {
    exchangeCodeForToken: jest.fn().mockReturnValue(of({ id: '1', name: 'Test User', email: 'test@example.com' })),
    getAndClearRedirectUrl: jest.fn().mockReturnValue('/dashboard')
  };
  
  beforeEach(async () => {
    // Mock localStorage
    Object.defineProperty(window, 'localStorage', {
      value: {
        getItem: jest.fn().mockImplementation((key) => {
          if (key === 'oauth_state') return 'test-state';
          return null;
        }),
        setItem: jest.fn(),
        removeItem: jest.fn()
      },
      writable: true
    });
    
    await TestBed.configureTestingModule({
      imports: [
        RouterTestingModule,
        HttpClientTestingModule,
        OAuthCallbackComponent
      ],
      providers: [
        { 
          provide: ActivatedRoute, 
          useValue: {
            queryParams: of({
              code: 'test-code',
              state: 'test-state'
            })
          }
        },
        { provide: AuthService, useValue: authServiceMock }
      ]
    }).compileComponents();
    
    router = TestBed.inject(Router);
    jest.spyOn(router, 'navigate');
    jest.spyOn(router, 'navigateByUrl');
    
    const fixture = TestBed.createComponent(OAuthCallbackComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });
  
  it('should create', () => {
    expect(component).toBeTruthy();
  });
  
  it('should exchange code for token', () => {
    expect(authServiceMock.exchangeCodeForToken).toHaveBeenCalledWith('test-code');
  });
  
  it('should navigate to redirect URL after successful auth', () => {
    expect(authServiceMock.getAndClearRedirectUrl).toHaveBeenCalled();
    expect(router.navigateByUrl).toHaveBeenCalledWith('/dashboard');
  });
  
  it('should handle error response from OAuth provider', async () => {
    // Re-create component with error query params
    await TestBed.resetTestingModule().configureTestingModule({
      imports: [
        RouterTestingModule,
        HttpClientTestingModule,
        OAuthCallbackComponent
      ],
      providers: [
        { 
          provide: ActivatedRoute, 
          useValue: {
            queryParams: of({
              error: 'access_denied'
            })
          }
        },
        { provide: AuthService, useValue: authServiceMock }
      ]
    }).compileComponents();
    
    router = TestBed.inject(Router);
    jest.spyOn(router, 'navigate');
    
    const fixture = TestBed.createComponent(OAuthCallbackComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
    
    expect(router.navigate).toHaveBeenCalledWith(['/sign-in'], { 
      queryParams: { error: expect.any(String) }
    });
  });
});
