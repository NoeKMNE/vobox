import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    redirect: '/login'
  },
  {
    path: '/login',
    name: 'Login',
    component: () => import('../components/Login.vue')
  },
  {
    path: '/onboarding/:step',
    name: 'Onboarding',
    component: () => import('../views/Onboarding.vue'),
    props: true,
    beforeEnter: (to) => {
      const step = Number(to.params.step)
      if (isNaN(step) || step < 1 || step > 3) {
        return { name: 'Onboarding', params: { step: 1 } }
      }
    }
  },
  {
    path: '/app',
    component: () => import('../layouts/AppShell.vue'),
    children: [
      {
        path: '',
        redirect: { name: 'Main' }
      },
      {
        path: 'main',
        name: 'Main',
        component: () => import('../views/MainScreen.vue')
      },
      {
        path: 'assistant',
        name: 'Assistant',
        component: () => import('../views/Assistant.vue')
      },
      {
        path: 'settings',
        name: 'Settings',
        component: () => import('../views/Settings.vue')
      },
      {
        path: 'fiche',
        name: 'Sheet',
        component: () => import('../views/Sheet.vue')
      },
      {
        path: 'data',
        name: 'Data',
        component: () => import('../views/Data.vue')
      },
      {
        path: 'archive',
        name: 'Archive',
        component: () => import('../views/Archive.vue')
      },
      {
        path: 'archive/producteur',
        name: 'ProducerHistory',
        component: () => import('../views/ProducerHistory.vue')
      },
      {
        path: 'rapport-semestre',
        name: 'RapportSemestre',
        component: () => import('../views/RapportSemestre.vue')
      },
      {
        path: 'exploitation/:id',
        name: 'Exploitation',
        component: () => import('../views/ExploitationView.vue'),
        props: true
      },
      {
        path: 'guide',
        name: 'Guide',
        component: () => import('../views/Guide.vue')
      }
    ]
  }
]

export const router = createRouter({
  history: createWebHistory(),
  routes
})