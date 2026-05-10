import {
  createRouter,
  createWebHistory,
  type NavigationGuardNext,
  type RouteLocationNormalized,
  type RouteRecordRaw,
} from 'vue-router';
import { useAuthStore } from './stores/auth';

const routes: RouteRecordRaw[] = [
  { path: '/setup', name: 'setup', component: () => import('./views/SetupView.vue') },
  { path: '/signin', name: 'signin', component: () => import('./views/SignInView.vue') },
  { path: '/register', name: 'register', component: () => import('./views/RegisterView.vue') },
  { path: '/pending', name: 'pending', component: () => import('./views/PendingView.vue') },
  { path: '/rejected', name: 'rejected', component: () => import('./views/RejectedView.vue') },
  { path: '/admin', name: 'admin', component: () => import('./views/AdminView.vue'), meta: { requireAdmin: true } },
  { path: '/dashboard', name: 'dashboard', component: () => import('./views/HomeView.vue'), meta: { requireApproved: true } },
  { path: '/keys', name: 'keys', component: () => import('./views/HomeView.vue'), meta: { requireApproved: true } },
  { path: '/', name: 'home', component: () => import('./views/HomeView.vue') },
];

export const router = createRouter({
  history: createWebHistory(),
  routes,
});

router.beforeEach(async (to: RouteLocationNormalized, _from, next: NavigationGuardNext) => {
  const auth = useAuthStore();
  // First time through, populate. Subsequent navigations reuse the store.
  if (auth.lastFetchAt == null) {
    await auth.boot();
  }

  // Setup gate.
  if (auth.needsSetup) {
    if (to.name === 'setup') return next();
    return next({ name: 'setup' });
  }
  if (!auth.needsSetup && to.name === 'setup') {
    return next({ name: 'home' });
  }

  const user = auth.user;
  // Signed-out access to /signin and /register is fine.
  if (!user) {
    if (to.meta.requireAdmin || to.meta.requireApproved) {
      return next({ name: 'signin', query: { next: to.fullPath } });
    }
    return next();
  }

  // Signed-in users can't visit /signin or /register.
  if (to.name === 'signin' || to.name === 'register') {
    return next({ name: 'home' });
  }

  // Status-route shaping.
  if (user.status === 'pending' && to.name !== 'pending') {
    return next({ name: 'pending' });
  }
  if (user.status === 'rejected' && to.name !== 'rejected') {
    return next({ name: 'rejected' });
  }

  if (to.meta.requireAdmin && user.role !== 'admin') {
    return next({ name: 'home' });
  }
  if (to.meta.requireApproved && user.status !== 'approved') {
    return next({ name: 'pending' });
  }
  next();
});

// Pinia store types augmentation: lastFetchAt added in stores/auth.ts but not
// exposed as a getter — re-augment a minimal interface here. Not strictly
// necessary; left for future readers.
declare module 'pinia' {
  // eslint-disable-next-line @typescript-eslint/no-empty-interface
  interface PiniaCustomProperties {}
}
