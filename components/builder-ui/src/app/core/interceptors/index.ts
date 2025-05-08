import { AuthInterceptor } from './auth.interceptor';
import { ErrorInterceptor } from './error.interceptor';
import { LoadingInterceptor } from './loading.interceptor';

// Export all interceptors for use with provideHttpClient(withInterceptors([...]))
export { AuthInterceptor, ErrorInterceptor, LoadingInterceptor };
