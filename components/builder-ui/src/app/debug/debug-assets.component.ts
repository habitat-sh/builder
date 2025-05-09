import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { AssetLoaderService, AssetStatus } from '../shared/services/asset-loader.service';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatDividerModule } from '@angular/material/divider';

@Component({
  selector: 'app-debug-assets',
  standalone: true,
  imports: [CommonModule, MatCardModule, MatButtonModule, MatIconModule, MatDividerModule],
  template: `
    <div class="debug-container">
      <mat-card>
        <mat-card-header>
          <mat-card-title>Asset Debug Tool</mat-card-title>
          <mat-card-subtitle>Check and fix image loading issues</mat-card-subtitle>
        </mat-card-header>
        
        <mat-card-content>
          <div class="controls">
            <button mat-raised-button color="primary" (click)="testAllAssets()">
              <mat-icon>refresh</mat-icon> Test All Assets
            </button>
            <button mat-raised-button color="accent" (click)="fixAssets()" [disabled]="!hasFailedAssets()">
              <mat-icon>build</mat-icon> Fix Failed Assets
            </button>
          </div>
          
          <h3>Current Assets</h3>
          <div class="image-grid">
            <mat-card class="image-item" *ngFor="let asset of assetPaths">
              <mat-card-header>
                <mat-card-title>{{ asset.name }}</mat-card-title>
              </mat-card-header>
              <div class="image-container">
                <img [src]="asset.path" [alt]="asset.name" (error)="onImageError(asset)" (load)="onImageLoad(asset)">
              </div>
              <mat-card-content>
                <div class="status" [ngClass]="{'success': asset.loaded, 'error': !asset.loaded && asset.attempted}">
                  {{ asset.loaded ? '✓ Loaded' : asset.attempted ? '✗ Failed' : 'Loading...' }}
                </div>
                <div class="path">{{ asset.path }}</div>
              </mat-card-content>
            </mat-card>
          </div>
          
          <h3>All Asset Requests</h3>
          <div class="asset-list">
            <div class="asset-item" *ngFor="let asset of allAssets">
              <span class="status-indicator" [ngClass]="asset.status"></span>
              <span class="asset-path">{{ asset.path }}</span>
              <span class="asset-status">{{ asset.status }}</span>
              <span class="asset-time">{{ asset.timestamp | date:'HH:mm:ss' }}</span>
            </div>
          </div>
        </mat-card-content>
      </mat-card>
    </div>
  `,
  styles: [`
    .debug-container {
      padding: 20px;
      max-width: 1200px;
      margin: 0 auto;
    }
    
    .controls {
      margin-bottom: 20px;
      display: flex;
      gap: 10px;
    }
    
    .image-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
      gap: 20px;
      margin-bottom: 30px;
    }
    
    .image-item {
      margin-bottom: 15px;
    }
    
    .image-container {
      height: 150px;
      display: flex;
      align-items: center;
      justify-content: center;
      background-color: #f5f5f5;
      margin: 10px 0;
    }
    
    .image-container img {
      max-width: 100%;
      max-height: 100%;
      object-fit: contain;
    }
    
    .status {
      font-weight: bold;
      margin-bottom: 5px;
    }
    
    .success {
      color: green;
    }
    
    .error {
      color: red;
    }
    
    .path {
      font-family: monospace;
      font-size: 12px;
      word-break: break-all;
      background: #f0f0f0;
      padding: 5px;
      border-radius: 3px;
      margin-top: 8px;
    }
    
    .asset-list {
      max-height: 300px;
      overflow-y: auto;
      border: 1px solid #eee;
      border-radius: 4px;
    }
    
    .asset-item {
      padding: 8px 12px;
      display: flex;
      align-items: center;
      border-bottom: 1px solid #eee;
      font-family: monospace;
      font-size: 14px;
    }
    
    .asset-item:last-child {
      border-bottom: none;
    }
    
    .status-indicator {
      width: 10px;
      height: 10px;
      border-radius: 50%;
      margin-right: 10px;
    }
    
    .status-indicator.loading {
      background-color: #FFC107;
    }
    
    .status-indicator.success {
      background-color: #4CAF50;
    }
    
    .status-indicator.error {
      background-color: #F44336;
    }
    
    .asset-path {
      flex: 1;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    
    .asset-status {
      margin: 0 15px;
      width: 70px;
    }
    
    .asset-time {
      color: #666;
    }
    
    h3 {
      margin: 20px 0 10px 0;
    }
  `]
})
export class DebugAssetsComponent implements OnInit {
  assetPaths = [
    { name: 'Habitat Logo', path: '/assets/images/habitat-logo.svg', loaded: false, attempted: false },
    { name: 'Habitat Logo (relative)', path: 'assets/images/habitat-logo.svg', loaded: false, attempted: false },
    { name: 'Builder Habitat Logo', path: '/assets/images/builder-habitat-logo.svg', loaded: false, attempted: false },
    { name: 'Builder Habitat Logo (relative)', path: 'assets/images/builder-habitat-logo.svg', loaded: false, attempted: false },
    { name: 'Avatar', path: '/assets/images/avatar.svg', loaded: false, attempted: false },
    { name: 'Avatar (relative)', path: 'assets/images/avatar.svg', loaded: false, attempted: false },
    { name: 'Habicat', path: '/assets/images/habicat.svg', loaded: false, attempted: false },
    { name: 'Habicat (relative)', path: 'assets/images/habicat.svg', loaded: false, attempted: false }
  ];
  
  allAssets: AssetStatus[] = [];
  
  constructor(private assetLoader: AssetLoaderService) {}
  
  ngOnInit(): void {
    // Subscribe to asset status changes
    this.assetLoader.assets$.subscribe(assets => {
      this.allAssets = assets;
    });
    
    // Initialize loading
    this.testAllAssets();
  }
  
  testAllAssets(): void {
    this.assetPaths.forEach(asset => {
      this.assetLoader.reportAssetLoading(asset.path);
      asset.attempted = false;
      asset.loaded = false;
    });
  }
  
  onImageLoad(asset: { path: string; loaded: boolean; attempted: boolean }): void {
    console.log(`Image loaded: ${asset.path}`);
    asset.loaded = true;
    asset.attempted = true;
    this.assetLoader.reportAssetSuccess(asset.path);
  }
  
  onImageError(asset: { path: string; loaded: boolean; attempted: boolean }): void {
    console.error(`Image failed to load: ${asset.path}`);
    asset.loaded = false;
    asset.attempted = true;
    this.assetLoader.reportAssetError(asset.path);
  }
  
  // Helper method to check if we have failed assets
  hasFailedAssets(): boolean {
    return this.assetPaths.some(asset => asset.attempted && !asset.loaded);
  }

  // Fix failed assets by copying any successful ones
  fixAssets(): void {
    const successfulAssets = this.assetPaths.filter(a => a.loaded);
    const failedAssets = this.assetPaths.filter(a => a.attempted && !a.loaded);
    
    if (successfulAssets.length > 0 && failedAssets.length > 0) {
      // For each failed asset, try to find a matching successful one
      failedAssets.forEach(failed => {
        const fixedPath = this.findMatchingSuccessfulAsset(failed.path, successfulAssets);
        if (fixedPath) {
          console.log(`Fixing ${failed.path} with ${fixedPath}`);
          failed.path = fixedPath;
          this.assetLoader.reportAssetLoading(failed.path);
        }
      });
    }
  }
  
  private findMatchingSuccessfulAsset(failedPath: string, successfulAssets: {path: string}[]): string | null {
    // Simple matching based on filename
    const filename = failedPath.split('/').pop();
    if (!filename) return null;
    
    const match = successfulAssets.find(asset => asset.path.includes(filename));
    return match ? match.path : null;
  }
}
