import {
  createConnectQueryKey,
  createProtobufSafeUpdater,
  useMutation,
} from '@connectrpc/connect-query'
import { useQueryClient } from '@tanstack/react-query'
import { useAtom, useSetAtom } from 'jotai'
import { useHydrateAtoms } from 'jotai/utils'
import { ScopeProvider } from 'jotai-scope'
import { ReactNode } from 'react'
import { DeepPartial } from 'react-hook-form'
import { match } from 'ts-pattern'

import { usePlanWithVersion } from '@/features/plans/hooks/usePlan'
import { CapacityForm } from '@/features/plans/pricecomponents/components/CapacityForm'
import { OneTimeForm } from '@/features/plans/pricecomponents/components/OneTimeForm'
import { RecurringForm } from '@/features/plans/pricecomponents/components/RecurringForm'
import { SlotsForm } from '@/features/plans/pricecomponents/components/SlotsForm'
import { SubscriptionRateForm } from '@/features/plans/pricecomponents/components/SubscriptionRateForm'
import { UsageBasedForm } from '@/features/plans/pricecomponents/components/UsageBasedForm'
import { addedComponentsAtom, editedComponentsAtom } from '@/features/plans/pricecomponents/utils'
import { mapFee } from '@/lib/mapping/feesToGrpc'
import { FormPriceComponent, PriceComponent, formPriceCompoentSchema } from '@/lib/schemas/plans'
import {
  createPriceComponent as createPriceComponentMutation,
  editPriceComponent as editPriceComponentMutation,
  listPriceComponents as listPriceComponentsQuery,
} from '@/rpc/api/pricecomponents/v1/pricecomponents-PriceComponentsService_connectquery'

import { componentFeeTypeAtom, componentNameAtom, editedComponentAtom } from './atoms'

interface CreatePriceComponentProps {
  createRef: string
  component: DeepPartial<PriceComponent>
}

export const CreatePriceComponent = ({ createRef, component }: CreatePriceComponentProps) => {
  const setAddedComponents = useSetAtom(addedComponentsAtom)

  const { version } = usePlanWithVersion()

  const queryClient = useQueryClient()

  const createPriceComponent = useMutation(createPriceComponentMutation, {
    onSuccess: data => {
      if (!version?.id) return
      setAddedComponents(components => components.filter(comp => comp.ref !== createRef))

      if (data.component) {
        queryClient.setQueryData(
          createConnectQueryKey(listPriceComponentsQuery, {
            planVersionId: version.id,
          }),
          createProtobufSafeUpdater(listPriceComponentsQuery, prev => ({
            components: [...(prev?.components ?? []), data.component!],
          }))
        )
      }
    },
  })

  const cancel = () => {
    // TODO confirm
    setAddedComponents(components => components.filter(comp => comp.ref !== createRef))
  }

  const onSubmit = (data: FormPriceComponent) => {
    const validated = formPriceCompoentSchema.safeParse(data)

    console.log('validated', validated)
    if (!version?.id) return

    createPriceComponent.mutate({
      planVersionId: version.id,
      // productId: undefined, // TODO
      name: data.name,
      fee: mapFee(data.fee),
    })
  }

  return (
    <ProviderWrapper init={component}>
      <PriceComponentForm cancel={cancel} onSubmit={onSubmit} />
    </ProviderWrapper>
  )
}

interface EditPriceComponentProps {
  component: PriceComponent
}

export const EditPriceComponent = ({ component }: EditPriceComponentProps) => {
  const setEditedComponents = useSetAtom(editedComponentsAtom)

  const { version } = usePlanWithVersion()

  const queryClient = useQueryClient()

  const editPriceComponent = useMutation(editPriceComponentMutation, {
    onSuccess: data => {
      if (!version?.id) return
      setEditedComponents(components => components.filter(compId => compId !== component.id))

      if (data.component) {
        queryClient.setQueryData(
          createConnectQueryKey(listPriceComponentsQuery, {
            planVersionId: version.id,
          }),
          createProtobufSafeUpdater(listPriceComponentsQuery, prev => {
            const idx = prev?.components?.findIndex(comp => comp.id === component.id) ?? -1
            if (idx === -1 || !data.component) return prev
            // now we update the componet it idx with the new data
            const updated = [...(prev?.components ?? [])]
            updated[idx] = data.component

            return {
              components: updated,
            }
          })
        )
      }
    },
  })

  const cancel = () => {
    // TODO confirm
    setEditedComponents(components => components.filter(comp => comp !== component.id))
  }

  const onSubmit = (data: FormPriceComponent) => {
    if (!version?.id) return
    editPriceComponent.mutate({
      planVersionId: version.id,
      component: {
        id: component.id,
        fee: mapFee(data.fee),
        name: data.name,
        productId: undefined, // TODO
      },
    })
  }

  return (
    <ProviderWrapper init={component}>
      <PriceComponentForm cancel={cancel} onSubmit={onSubmit} />
    </ProviderWrapper>
  )
}

const ProviderWrapper = ({
  children,
  init,
}: {
  children: ReactNode
  init: DeepPartial<PriceComponent>
}) => {
  return (
    <ScopeProvider atoms={[editedComponentAtom]}>
      <HydrateAtoms initialValues={init}>{children}</HydrateAtoms>
    </ScopeProvider>
  )
}

interface PriceComponentFormProps {
  cancel: () => void
  onSubmit: (data: FormPriceComponent) => void
}

const PriceComponentForm = ({ cancel, onSubmit: _onSubmit }: PriceComponentFormProps) => {
  const [feeType] = useAtom(componentFeeTypeAtom)
  const [name] = useAtom(componentNameAtom)

  const onSubmit = (data: FormPriceComponent['fee']['data']) => {
    _onSubmit({ fee: { fee: feeType!, data } as FormPriceComponent['fee'], name: name! })
  }

  return match<typeof feeType, ReactNode>(feeType)
    .with('rate', () => <SubscriptionRateForm cancel={cancel} onSubmit={onSubmit} />)
    .with('slot', () => <SlotsForm cancel={cancel} onSubmit={onSubmit} />)
    .with('capacity', () => <CapacityForm cancel={cancel} onSubmit={onSubmit} />)
    .with('usage', () => <UsageBasedForm cancel={cancel} onSubmit={onSubmit} />)
    .with('extraRecurring', () => <RecurringForm cancel={cancel} onSubmit={onSubmit} />)
    .with('oneTime', () => <OneTimeForm cancel={cancel} onSubmit={onSubmit} />)
    .otherwise(() => <div>Unknown fee type. Please contact support</div>)
}

const HydrateAtoms = ({
  initialValues,
  children,
}: {
  initialValues: DeepPartial<PriceComponent>
  children: ReactNode
}) => {
  useHydrateAtoms([[editedComponentAtom, initialValues]])
  return children
}
