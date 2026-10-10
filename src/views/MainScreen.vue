<script setup lang="ts">
import {
  Archive,
  BookOpenCheck,
  Database,
  FileText,
  Settings,
  Sparkles
} from '@lucide/vue'
import type { Component } from 'vue'

interface DashboardFeature {
  title: string
  description: string
  route: string
  icon: Component
}

const today = new Intl.DateTimeFormat('fr-FR', {
  weekday: 'long',
  day: 'numeric',
  month: 'long',
  year: 'numeric'
}).format(new Date())

const features: DashboardFeature[] = [
  {
    title: 'Assistant IA',
    description: 'Posez vos questions et pilotez vos données avec votre assistant agricole.',
    route: 'Assistant',
    icon: Sparkles
  },
  {
    title: 'Fiches de traçabilité',
    description: 'Créez et consultez les fiches de collecte de vos produits.',
    route: 'Sheet',
    icon: FileText
  },
  {
    title: 'Données des exploitations',
    description: 'Recherchez les exploitations et consultez leurs informations.',
    route: 'Data',
    icon: Database
  },
  {
    title: 'Archives',
    description: 'Retrouvez les fiches enregistrées et les opérations précédentes.',
    route: 'Archive',
    icon: Archive
  },
  {
    title: 'Historique producteurs',
    description: 'Consultez l’historique des activités de vos producteurs.',
    route: 'ProducerHistory',
    icon: Database
  },
  {
    title: 'Rapport semestriel',
    description: 'Accédez au bilan des produits et des restes de fin de semestre.',
    route: 'RapportSemestre',
    icon: FileText
  },
  {
    title: 'Guide de prise en main',
    description: 'Découvrez les outils et les étapes pour bien démarrer.',
    route: 'Guide',
    icon: BookOpenCheck
  },
  {
    title: 'Paramètres',
    description: 'Personnalisez l’application et gérez vos préférences.',
    route: 'Settings',
    icon: Settings
  }
]
</script>

<template>
  <div class="dashboard-page">
    <header class="dashboard-header">
      <div>
        <p class="dashboard-eyebrow">ESPACE DE TRAVAIL</p>
        <h1>Bonjour, bienvenue sur Vobox</h1>
        <p class="dashboard-intro">
          Retrouvez vos outils au même endroit et poursuivez votre travail en un clic.
        </p>
      </div>
      <span class="dashboard-date">{{ today }}</span>
    </header>

    <section class="features-section" aria-labelledby="features-heading">
      <div class="section-heading">
        <div>
          <h2 id="features-heading">Toutes vos fonctionnalités</h2>
          <p>Choisissez un espace pour commencer.</p>
        </div>
        <span class="feature-count">{{ features.length }} espaces</span>
      </div>

      <div class="feature-grid">
        <RouterLink
          v-for="feature in features"
          :key="feature.route"
          :to="{ name: feature.route }"
          class="feature-card"
        >
          <span class="feature-icon">
            <component :is="feature.icon" :size="21" />
          </span>
          <span class="feature-copy">
            <span class="feature-title">{{ feature.title }}</span>
            <span class="feature-description">{{ feature.description }}</span>
          </span>
          <span class="feature-action">Ouvrir <span aria-hidden="true">→</span></span>
        </RouterLink>
      </div>
    </section>
  </div>
</template>

<style scoped>
.dashboard-page {
  box-sizing: border-box;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: clamp(24px, 4vw, 52px);
  color: var(--on-surface);
}

.dashboard-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 24px;
  margin: 0 auto 42px;
  max-width: 1180px;
}

.dashboard-eyebrow {
  margin: 0 0 10px;
  color: var(--primary);
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 1.2px;
}

.dashboard-header h1 {
  margin: 0;
  font-size: clamp(28px, 4vw, 40px);
  font-weight: 700;
  letter-spacing: -0.6px;
  line-height: 1.15;
}

.dashboard-intro {
  max-width: 620px;
  margin: 12px 0 0;
  color: var(--on-surface-variant);
  font-size: 15px;
  line-height: 1.6;
}

.dashboard-date {
  flex-shrink: 0;
  padding: 10px 16px;
  border: 1px solid var(--outline-variant);
  border-radius: var(--shape-full);
  background: var(--surface-container-lowest);
  color: var(--on-surface-variant);
  font-size: 13px;
  font-weight: 500;
  text-transform: capitalize;
}

.features-section {
  max-width: 1180px;
  margin: 0 auto;
}

.section-heading {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
}

.section-heading h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 700;
}

.section-heading p {
  margin: 5px 0 0;
  color: var(--on-surface-variant);
  font-size: 14px;
}

.feature-count {
  flex-shrink: 0;
  color: var(--on-surface-variant);
  font-size: 13px;
}

.feature-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, 230px), 1fr));
  gap: 14px;
}

.feature-card {
  min-height: 206px;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding: 20px;
  border: 1px solid var(--outline-variant);
  border-radius: var(--shape-xl);
  background: var(--surface-container-lowest);
  color: inherit;
  text-decoration: none;
  transition:
    transform var(--motion),
    border-color var(--motion),
    box-shadow var(--motion),
    background var(--motion);
}

.feature-card:hover {
  transform: translateY(-2px);
  border-color: var(--primary);
  background: var(--surface-container-low);
  box-shadow: var(--elev-1);
}

.feature-card:focus-visible {
  outline: 3px solid var(--primary);
  outline-offset: 3px;
}

.feature-icon {
  width: 44px;
  height: 44px;
  display: grid;
  place-items: center;
  border-radius: var(--shape-md);
  background: var(--primary-container);
  color: var(--on-primary-container);
}

.feature-copy {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 16px 0;
}

.feature-title {
  font-size: 16px;
  font-weight: 650;
}

.feature-description {
  color: var(--on-surface-variant);
  font-size: 13px;
  line-height: 1.5;
}

.feature-action {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  margin-top: auto;
  color: var(--primary);
  font-size: 13px;
  font-weight: 700;
}

@media (max-width: 700px) {
  .dashboard-page {
    padding: 24px 18px;
  }

  .dashboard-header {
    flex-direction: column;
    margin-bottom: 30px;
  }
}

@media (max-width: 420px) {
  .section-heading {
    align-items: flex-start;
    flex-direction: column;
  }

  .feature-card {
    min-height: 180px;
  }
}
</style>
