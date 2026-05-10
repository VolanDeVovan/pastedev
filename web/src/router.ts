import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router';
import HelloView from './views/HelloView.vue';

const routes: RouteRecordRaw[] = [
  { path: '/', name: 'home', component: HelloView },
];

export const router = createRouter({
  history: createWebHistory(),
  routes,
});
