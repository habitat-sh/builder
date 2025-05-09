import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';

export interface AssetStatus {
  path: string;
  status: 'loading' | 'success' | 'error';
  timestamp: number;
}

@Injectable({
  providedIn: 'root'
})
export class AssetLoaderService {
  private assetStatusMap = new Map<string, AssetStatus>();
  private assetsSubject = new BehaviorSubject<AssetStatus[]>([]);
  
  assets$ = this.assetsSubject.asObservable();
  
  constructor() { }
  
  reportAssetLoading(path: string): void {
    this.updateAssetStatus(path, 'loading');
  }
  
  reportAssetSuccess(path: string): void {
    this.updateAssetStatus(path, 'success');
  }
  
  reportAssetError(path: string): void {
    this.updateAssetStatus(path, 'error');
  }
  
  private updateAssetStatus(path: string, status: 'loading' | 'success' | 'error'): void {
    this.assetStatusMap.set(path, {
      path,
      status,
      timestamp: Date.now()
    });
    
    // Update the observable with the current status of all assets
    this.assetsSubject.next(Array.from(this.assetStatusMap.values()));
  }
  
  getAssetStatus(path: string): AssetStatus | undefined {
    return this.assetStatusMap.get(path);
  }
  
  getAllAssetStatus(): AssetStatus[] {
    return Array.from(this.assetStatusMap.values());
  }
}
