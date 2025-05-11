import { Injectable } from '@angular/core';
import { HttpRequest, HttpResponse, HttpHandler, HttpEvent, HttpInterceptor } from '@angular/common/http';
import { Observable, of } from 'rxjs';
import { delay } from 'rxjs/operators';

/**
 * Mock interceptor for package-related API requests
 * This interceptor provides mock data for development/testing
 */
@Injectable()
export class MockPackageApiInterceptor implements HttpInterceptor {
  
  constructor() {}
  
  intercept(request: HttpRequest<any>, next: HttpHandler): Observable<HttpEvent<any>> {
    // Only intercept package-related requests if they contain 'pkgs'
    if (request.url.includes('/pkgs/')) {
      return this.handlePackageRequest(request);
    }
    
    // Pass through any other requests
    return next.handle(request);
  }
  
  /**
   * Handles package-related requests
   */
  private handlePackageRequest(request: HttpRequest<any>): Observable<HttpEvent<any>> {
    const url = request.url;
    
    // Package search
    if (url.includes('/pkgs/search')) {
      return this.handlePackageSearch(request);
    }
    
    // Package details (matches /pkgs/:origin/:name/:version?/:release?)
    const detailsMatch = url.match(/\/pkgs\/([^\/]+)\/([^\/]+)(\/([^\/]+)(\/([^\/]+))?)?$/);
    if (detailsMatch) {
      const origin = detailsMatch[1];
      const name = detailsMatch[2];
      const version = detailsMatch[4];
      const release = detailsMatch[6];
      
      return this.handlePackageDetails(origin, name, version, release);
    }
    
    // Channels for a package
    const channelsMatch = url.match(/\/pkgs\/([^\/]+)\/([^\/]+)\/channels$/);
    if (channelsMatch) {
      const origin = channelsMatch[1];
      const name = channelsMatch[2];
      
      return this.handlePackageChannels(origin, name);
    }
    
    // Return a 404 for unhandled package routes
    return of(new HttpResponse({
      status: 404,
      statusText: 'Not Found',
      body: { message: 'API endpoint not found' }
    }));
  }
  
  /**
   * Handles package search requests
   */
  private handlePackageSearch(request: HttpRequest<any>): Observable<HttpEvent<any>> {
    // Extract query parameters
    const params = new URLSearchParams(request.params.toString());
    const query = params.get('query') || '';
    const origin = params.get('origin') || '';
    const platform = params.get('platform') || '';
    const page = Number(params.get('page') || '1');
    const limit = Number(params.get('limit') || '20');
    
    // Create mock data
    const mockPackages = this.generateMockPackages();
    
    // Filter based on search parameters
    let filteredPackages = [...mockPackages];
    
    if (origin) {
      filteredPackages = filteredPackages.filter(pkg => pkg.origin.toLowerCase() === origin.toLowerCase());
    }
    
    if (query) {
      const searchQuery = query.toLowerCase();
      filteredPackages = filteredPackages.filter(pkg => 
        pkg.name.toLowerCase().includes(searchQuery) || 
        pkg.description?.toLowerCase().includes(searchQuery)
      );
    }
    
    if (platform) {
      filteredPackages = filteredPackages.filter(pkg => 
        pkg.platforms.some(plt => plt.toLowerCase().includes(platform.toLowerCase()))
      );
    }
    
    // Sort by updatedAt (newest first)
    filteredPackages = filteredPackages.sort((a, b) => {
      const dateA = new Date(a.updatedAt);
      const dateB = new Date(b.updatedAt);
      return dateB.getTime() - dateA.getTime();
    });
    
    // Paginate results
    const startIndex = (page - 1) * limit;
    const endIndex = page * limit;
    const paginatedPackages = filteredPackages.slice(startIndex, endIndex);
    
    // Return mock response
    return of(new HttpResponse({
      status: 200,
      body: {
        packages: paginatedPackages,
        totalCount: filteredPackages.length,
        perPage: limit,
        page: page
      }
    })).pipe(delay(800)); // Add delay to simulate network latency
  }
  
  /**
   * Handles package details requests
   */
  private handlePackageDetails(origin: string, name: string, version?: string, release?: string): Observable<HttpEvent<any>> {
    // Find the matching package in our mock data
    const mockPackages = this.generateMockPackages();
    let matchingPackage = mockPackages.find(pkg => 
      pkg.origin === origin && 
      pkg.name === name
    );
    
    if (!matchingPackage) {
      // Return 404 if package not found
      return of(new HttpResponse({
        status: 404,
        statusText: 'Not Found',
        body: { message: 'Package not found' }
      }));
    }
    
    // Create a detailed version of the package with additional fields
    const detailedPackage = {
      ...matchingPackage,
      maintainer: 'The Habitat Maintainers <humans@habitat.sh>',
      license: 'Apache-2.0',
      dependencies: [
        { origin: 'core', name: 'glibc' },
        { origin: 'core', name: 'gcc-libs' },
        { origin: 'core', name: 'openssl' }
      ],
      installedSize: 1024 * 1024 * 15, // 15MB
      manifestSize: 1024 * 10, // 10KB
      target: 'x86_64-linux',
      fullName: `${origin}/${name}/${version || '1.0.0'}/${release || '20200101000000'}`,
      buildDependencies: [
        { origin: 'core', name: 'rust' },
        { origin: 'core', name: 'gcc' }
      ],
      runDependencies: [
        { origin: 'core', name: 'glibc' },
        { origin: 'core', name: 'gcc-libs' }
      ]
    };
    
    // Return mock response
    return of(new HttpResponse({
      status: 200,
      body: detailedPackage
    })).pipe(delay(800)); // Add delay to simulate network latency
  }
  
  /**
   * Handles package channels requests
   */
  private handlePackageChannels(origin: string, name: string): Observable<HttpEvent<any>> {
    // Create mock channels
    const mockChannels = [
      {
        id: 'channel-1',
        name: 'stable',
        packages: 10,
        promotable: true,
        isDefault: true
      },
      {
        id: 'channel-2',
        name: 'unstable',
        packages: 15,
        promotable: true,
        isDefault: false
      },
      {
        id: 'channel-3',
        name: 'dev',
        packages: 8,
        promotable: true,
        isDefault: false
      }
    ];
    
    // Return mock response
    return of(new HttpResponse({
      status: 200,
      body: mockChannels
    })).pipe(delay(600)); // Add delay to simulate network latency
  }
  
  /**
   * Generates mock package data
   */
  private generateMockPackages() {
    return [
      {
        id: 'pkg-1',
        name: 'hab',
        origin: 'core',
        version: '1.6.412',
        release: '20220517110908',
        latestVersion: '1.6.412',
        latestRelease: '20220517110908',
        platforms: ['x86_64-linux', 'x86_64-windows'],
        description: 'The Habitat Supervisor - habitat is an application automation framework that allows you to build, deploy, and manage applications in a behavior-driven way.',
        visibility: 'public',
        downloadCount: 12345,
        updatedAt: '2022-05-17T11:09:08.000Z',
        channels: ['stable', 'unstable']
      },
      {
        id: 'pkg-2',
        name: 'nginx',
        origin: 'core',
        version: '1.19.6',
        release: '20220304205114',
        latestVersion: '1.19.6',
        latestRelease: '20220304205114',
        platforms: ['x86_64-linux', 'x86_64-windows'],
        description: 'NGINX web server - nginx [engine x] is an HTTP and reverse proxy server, a mail proxy server, and a generic TCP/UDP proxy server.',
        visibility: 'public',
        downloadCount: 9876,
        updatedAt: '2022-03-04T20:51:14.000Z',
        channels: ['stable', 'unstable']
      },
      {
        id: 'pkg-3',
        name: 'postgresql',
        origin: 'core',
        version: '13.2',
        release: '20220115124059',
        latestVersion: '13.2',
        latestRelease: '20220115124059',
        platforms: ['x86_64-linux'],
        description: 'PostgreSQL database server - PostgreSQL is a powerful, open source object-relational database system.',
        visibility: 'public',
        downloadCount: 7654,
        updatedAt: '2022-01-15T12:40:59.000Z',
        channels: ['stable', 'unstable']
      },
      {
        id: 'pkg-4',
        name: 'redis',
        origin: 'core',
        version: '6.2.6',
        release: '20220214091145',
        latestVersion: '6.2.6',
        latestRelease: '20220214091145',
        platforms: ['x86_64-linux'],
        description: 'Redis in-memory data structure store - Redis is an in-memory data structure store, used as a database, cache and message broker.',
        visibility: 'public',
        downloadCount: 5432,
        updatedAt: '2022-02-14T09:11:45.000Z',
        channels: ['stable', 'unstable']
      },
      {
        id: 'pkg-5',
        name: 'node',
        origin: 'core',
        version: '14.16.0',
        release: '20210405103022',
        latestVersion: '16.14.2',
        latestRelease: '20220412134523',
        platforms: ['x86_64-linux', 'x86_64-windows'],
        description: 'Node.js JavaScript runtime - Node.js is a JavaScript runtime built on Chrome\'s V8 JavaScript engine.',
        visibility: 'public',
        downloadCount: 4321,
        updatedAt: '2022-04-12T13:45:23.000Z',
        channels: ['stable', 'unstable']
      },
      {
        id: 'pkg-6',
        name: 'mysql',
        origin: 'core',
        version: '8.0.23',
        release: '20220128091023',
        latestVersion: '8.0.23',
        latestRelease: '20220128091023',
        platforms: ['x86_64-linux'],
        description: 'MySQL database server - MySQL is a widely used, open-source relational database management system.',
        visibility: 'public',
        downloadCount: 3210,
        updatedAt: '2022-01-28T09:10:23.000Z',
        channels: ['stable']
      },
      {
        id: 'pkg-7',
        name: 'prometheus',
        origin: 'core',
        version: '2.26.0',
        release: '20220221155948',
        latestVersion: '2.26.0',
        latestRelease: '20220221155948',
        platforms: ['x86_64-linux'],
        description: 'Prometheus monitoring system - Prometheus is an open-source systems monitoring and alerting toolkit.',
        visibility: 'public',
        downloadCount: 2109,
        updatedAt: '2022-02-21T15:59:48.000Z',
        channels: ['stable', 'unstable']
      },
      {
        id: 'pkg-8',
        name: 'docker',
        origin: 'core',
        version: '20.10.5',
        release: '20220118124512',
        latestVersion: '20.10.5',
        latestRelease: '20220118124512',
        platforms: ['x86_64-linux'],
        description: 'Docker container runtime - Docker is a platform for developers and sysadmins to build, share, and run applications with containers.',
        visibility: 'public',
        downloadCount: 1987,
        updatedAt: '2022-01-18T12:45:12.000Z',
        channels: ['stable', 'unstable']
      },
      {
        id: 'pkg-9',
        name: 'consul',
        origin: 'core',
        version: '1.9.4',
        release: '20220304123015',
        latestVersion: '1.9.4',
        latestRelease: '20220304123015',
        platforms: ['x86_64-linux', 'x86_64-windows'],
        description: 'Consul service mesh - Consul is a service networking solution that enables teams to manage secure network connectivity between services and across multi-cloud environments.',
        visibility: 'public',
        downloadCount: 1543,
        updatedAt: '2022-03-04T12:30:15.000Z',
        channels: ['stable']
      },
      {
        id: 'pkg-10',
        name: 'vault',
        origin: 'core',
        version: '1.6.3',
        release: '20220215083045',
        latestVersion: '1.6.3',
        latestRelease: '20220215083045',
        platforms: ['x86_64-linux', 'x86_64-windows'],
        description: 'HashiCorp Vault - Vault is a tool for securely accessing secrets via a unified interface and tight access control.',
        visibility: 'public',
        downloadCount: 1210,
        updatedAt: '2022-02-15T08:30:45.000Z',
        channels: ['stable', 'unstable']
      },
      {
        id: 'pkg-11',
        name: 'python',
        origin: 'core',
        version: '3.9.2',
        release: '20220125143022',
        latestVersion: '3.9.2',
        latestRelease: '20220125143022',
        platforms: ['x86_64-linux', 'x86_64-windows'],
        description: 'Python programming language - Python is a programming language that lets you work quickly and integrate systems more effectively.',
        visibility: 'public',
        downloadCount: 9870,
        updatedAt: '2022-01-25T14:30:22.000Z',
        channels: ['stable', 'unstable']
      },
      {
        id: 'pkg-12',
        name: 'ruby',
        origin: 'core',
        version: '3.0.1',
        release: '20220210102233',
        latestVersion: '3.0.1',
        latestRelease: '20220210102233',
        platforms: ['x86_64-linux', 'x86_64-windows'],
        description: 'Ruby programming language - A dynamic, open source programming language with a focus on simplicity and productivity.',
        visibility: 'public',
        downloadCount: 7651,
        updatedAt: '2022-02-10T10:22:33.000Z',
        channels: ['stable']
      }
    ];
  }
}
