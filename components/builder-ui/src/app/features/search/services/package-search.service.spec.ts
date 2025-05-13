import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { of } from 'rxjs';

import { PackageSearchService } from './package-search.service';
import { ApiService } from '../../../core/services/api.service';
import { PackageVisibility, PackageType } from '../../../shared/models/package.model';

describe('PackageSearchService', () => {
  let service: PackageSearchService;
  let httpMock: HttpTestingController;
  let mockApiService: jasmine.SpyObj<ApiService>;

  beforeEach(() => {
    mockApiService = jasmine.createSpyObj('ApiService', ['get']);

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        PackageSearchService,
        { provide: ApiService, useValue: mockApiService }
      ]
    });
    
    service = TestBed.inject(PackageSearchService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('searchPackages', () => {
    it('should call API service with correct parameters', () => {
      const mockResponse = {
        results: [
          {
            id: '1',
            ident: {
              origin: 'core',
              name: 'nginx',
              version: '1.19.3',
              release: '20201014122127'
            },
            origin: 'core',
            name: 'nginx',
            version: '1.19.3',
            release: '20201014122127',
            visibility: PackageVisibility.Public,
            type: PackageType.Standard,
            channels: [],
            platforms: ['x86_64-linux'],
            target: {
              name: 'x86_64-linux',
              platform: 'linux',
              architecture: 'x86_64'
            },
            checksum: 'abcd1234',
            manifest: '',
            config: '',
            deps: [],
            tdeps: [],
            buildDeps: [],
            buildTdeps: [],
            exposes: [],
            createdAt: new Date(),
            updatedAt: new Date(),
            ownerId: '123'
          }
        ],
        totalCount: 1,
        nextRange: 50
      };
      
      mockApiService.get.and.returnValue(of(mockResponse));
      
      service.searchPackages('core', 'nginx').subscribe(response => {
        expect(response).toEqual(mockResponse);
      });
      
      expect(mockApiService.get).toHaveBeenCalledWith(
        'packages/search', 
        { origin: 'core', query: 'nginx', distinct: 'true' },
        { params: { range: '0', limit: '50' } }
      );
    });
  });

  describe('packageString', () => {
    it('should format a full package correctly', () => {
      const pkg = {
        origin: 'core',
        name: 'nginx',
        version: '1.19.3',
        release: '20201014122127'
      };
      
      expect(service.packageString(pkg)).toBe('core/nginx/1.19.3/20201014122127');
    });
    
    it('should handle partial packages correctly', () => {
      const pkg = {
        origin: 'core',
        name: 'nginx'
      };
      
      expect(service.packageString(pkg)).toBe('core/nginx');
    });
  });
});
