{%- match func.return_type() -%}
{%- when Some with (return_type) %}
{%- if func.is_async() %}
public func {{ func.name()|fn_name }}({%- call swift::arg_list_decl(func) -%}) async throws {% call swift::throws(func) %} -> {{ return_type|type_name }} {

   class CbWrapper {
        var cb: (Result<{{ return_type|lift_fn }}, Never>) -> ()
        init(cb: @escaping (Result<{{ return_type|lift_fn }}, Never>) -> ()) {
            self.cb = cb
        }
    }
    let ffi_future = {% call swift::to_ffi_call(func) %};
    let task = Task { operation: () -> [{{ return_type|lift_fn }}] in 
        ffi_future.run()
    };
    func onComplete(cbWrapperPtr: UnsafeMutableRawPointer?, rustFnRetVal: __swift_bridge__${{ return_type|lift_fn }}) {
        let wrapper = Unmanaged<CbWrapper>.fromOpaque(cbWrapperPtr!).takeRetainedValue()
        wrapper.cb(.success(rustFnRetVal.intoSwiftRepr()))
    }
    return await withCheckedContinuation({ (continuation: CheckedContinuation<{{ return_type|lift_fn }}, Never>) in
        let callback = { rustFnRetVal in
            continuation.resume(with: rustFnRetVal)
        }
        let wrapper = CbWrapper(cb: callback)
        let wrapperPtr = Unmanaged.passRetained(wrapper).toOpaque()
        __swift_bridge__$some_function(wrapperPtr, onComplete)
    })
    return 
}


{%- else %}
public func {{ func.name()|fn_name }}({%- call swift::arg_list_decl(func) -%}) {% call swift::throws(func) %} -> {{ return_type|type_name }} {
    return {% call swift::try(func) %} {{ return_type|lift_fn }}(
        {% call swift::to_ffi_call(func) %}
    )
}
{% endif %}

{% when None %}

public func {{ func.name()|fn_name }}({% call swift::arg_list_decl(func) %}) {% call swift::throws(func) %} {
    {% call swift::to_ffi_call(func) %}
}
{% endmatch %}
