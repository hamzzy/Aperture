import type {ReactNode} from 'react';
import clsx from 'clsx';
import Heading from '@theme/Heading';
import styles from './styles.module.css';

type FeatureItem = {
  title: string;
  description: ReactNode;
};

const FeatureList: FeatureItem[] = [
  {
    title: 'eBPF-Powered Profiling',
    description: (
      <>
        CPU sampling, lock contention tracing, and syscall analysis with less
        than 1% overhead using kernel-level eBPF programs. No code changes or
        restarts required.
      </>
    ),
  },
  {
    title: 'Distributed Architecture',
    description: (
      <>
        Agent-aggregator model scales from a single host to Kubernetes clusters.
        gRPC transport, ClickHouse storage, and Prometheus metrics built in.
      </>
    ),
  },
  {
    title: 'Interactive Dashboard',
    description: (
      <>
        Web UI with interactive flamegraphs, top functions, syscall histograms,
        differential profiling, and configurable alerts. Export to JSON or
        collapsed-stack format.
      </>
    ),
  },
];

function Feature({title, description}: FeatureItem) {
  return (
    <div className={clsx('col col--4')}>
      <div className="text--center padding-horiz--md" style={{paddingTop: '2rem'}}>
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures(): ReactNode {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
