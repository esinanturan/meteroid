import { createConnectQueryKey, useMutation } from '@connectrpc/connect-query'
import { Form, InputFormField, Modal } from '@md/ui'
import { useQueryClient } from '@tanstack/react-query'
import { ComponentProps } from 'react'
import { toast } from 'sonner'
import { z } from 'zod'

import { balanceSchema } from '@/features/customers/cards/balance/schema'
import { useZodForm } from '@/hooks/useZodForm'
import {
  getCustomerById,
  updateCustomer,
} from '@/rpc/api/customers/v1/customers-CustomersService_connectquery'
import { Customer } from '@/rpc/api/customers/v1/models_pb'

type Props = Pick<ComponentProps<typeof Modal>, 'visible' | 'onCancel'> & {
  customer: Customer
}

export const EditBalanceModal = ({customer, ...props}: Props) => {
  const queryClient = useQueryClient()
  const updateCustomerMutation = useMutation(updateCustomer, {
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: createConnectQueryKey(getCustomerById, {id: customer.id}),
      })
    },
  })

  const methods = useZodForm({
    mode: 'onChange',
    schema: balanceSchema,
    defaultValues: {
      balanceValueCents: Number(customer.balanceValueCents),
    },
  })

  const onSubmit = async (data: z.infer<typeof balanceSchema>) => {
    console.log('data', data)
    await updateCustomerMutation.mutateAsync({
      customer: {
        id: customer.id,
        balanceValueCents: BigInt(data.balanceValueCents),
      },
    })
    toast.success('Balance updated')
    props.onCancel?.()
  }

  return (
    <Modal
      size="small"
      header={<>Edit balance</>}
      {...props}
      onConfirm={() => methods.handleSubmit(onSubmit)()}
    >
      <Modal.Content>
        <Form {...methods}>
          <form>
            <div className="py-4 w-full space-y-4">
              <InputFormField
                label="Balance"
                name="balanceValueCents"
                type="number"
                control={methods.control}
              />
            </div>
          </form>
        </Form>
      </Modal.Content>
    </Modal>
  )
}
